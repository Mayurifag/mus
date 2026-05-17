use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Stdio,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use axum::{
    body::{Body, Bytes},
    extract::Query,
    http::header,
    response::Response,
    Json,
};
use futures_util::{stream, StreamExt};
use reqwest::Url;
use serde::Deserialize;
use tokio::{fs, process::Command};

use crate::{
    error::AppError,
    models::{ArtworkSearchQuery, ArtworkSearchResult},
    util::{now_nanos, run_command_status},
};

const USER_AGENT: &str = "mus/0.1 artwork search";
const ARTWORK_CACHE_TTL: Duration = Duration::from_secs(600);
type ArtworkCacheEntry = (Instant, Vec<ArtworkSearchResult>);
type ArtworkCache = Mutex<HashMap<String, ArtworkCacheEntry>>;
static ARTWORK_CACHE: OnceLock<ArtworkCache> = OnceLock::new();

pub async fn search_artwork(
    Query(query): Query<ArtworkSearchQuery>,
) -> Result<Json<Vec<ArtworkSearchResult>>, AppError> {
    let results = ArtworkProviders::default().search(&query).await?;
    Ok(Json(results))
}

pub async fn stream_artwork(Query(query): Query<ArtworkSearchQuery>) -> Result<Response, AppError> {
    let stream = async_stream::stream! {
        let cache_key = artwork_cache_key(&query);
        if let Some(results) = cached_artwork_results(&cache_key) {
            yield Ok::<Bytes, std::convert::Infallible>(artwork_chunk(results));
            return;
        }

        let mut results = Vec::new();
        match search_itunes_quick(&query).await {
            Ok(mut quick_results) => {
                results.append(&mut quick_results);
                let sorted = dedupe_and_sort(results.clone(), &query);
                if !sorted.is_empty() {
                    yield Ok(artwork_chunk(sorted));
                }
            }
            Err(error) => tracing::warn!(provider = "iTunes", %error, "artwork provider failed"),
        }

        match search_itunes_artist_catalog(&query).await {
            Ok(mut artist_results) => {
                results.append(&mut artist_results);
                let sorted = dedupe_and_sort(results.clone(), &query);
                if !sorted.is_empty() {
                    yield Ok(artwork_chunk(sorted));
                }
            }
            Err(error) => tracing::warn!(provider = "iTunes", %error, "artwork provider failed"),
        }

        if results.len() < 4 && !query_contains_apple_music_url(&query) {
            match search_cover_art_archive(&query).await {
                Ok(mut caa_results) => {
                    results.append(&mut caa_results);
                    let sorted = dedupe_and_sort(results.clone(), &query);
                    if !sorted.is_empty() {
                        yield Ok(artwork_chunk(sorted));
                    }
                }
                Err(error) => tracing::warn!(provider = "Cover Art Archive", %error, "artwork provider failed"),
            }
        }

        let results = dedupe_and_sort(results, &query);
        cache_artwork_results(cache_key, results);
    };

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .body(Body::from_stream(stream))
        .map_err(|error| anyhow!(error))?)
}

pub async fn download_artwork_as_jpeg(url: &str, cache_dir: PathBuf) -> Result<PathBuf> {
    fs::create_dir_all(&cache_dir).await?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?;
    let suffix = now_nanos();
    let source = cache_dir.join(format!("artwork-{suffix}"));
    let jpeg = cache_dir.join(format!("artwork-{suffix}.jpg"));
    fs::write(&source, bytes).await?;

    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-i"])
        .arg(&source)
        .args(["-frames:v", "1", "-q:v", "2"])
        .arg(&jpeg)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = run_command_status(command, Duration::from_secs(60)).await;
    let _ = fs::remove_file(source).await;

    let status = status?;

    if !status.success() || !jpeg.is_file() {
        let _ = fs::remove_file(&jpeg).await;
        return Err(anyhow!("failed to convert artwork to JPEG"));
    }

    Ok(jpeg)
}

struct ArtworkProviders {
    providers: Vec<ArtworkProvider>,
}

impl Default for ArtworkProviders {
    fn default() -> Self {
        Self {
            providers: vec![ArtworkProvider::ITunes, ArtworkProvider::CoverArtArchive],
        }
    }
}

impl ArtworkProviders {
    async fn search(&self, query: &ArtworkSearchQuery) -> Result<Vec<ArtworkSearchResult>> {
        let cache_key = artwork_cache_key(query);
        if let Some(results) = cached_artwork_results(&cache_key) {
            return Ok(results);
        }

        let mut results = Vec::new();
        for provider in &self.providers {
            match provider.search(query).await {
                Ok(mut provider_results) => {
                    results.append(&mut provider_results);
                    if matches!(provider, ArtworkProvider::ITunes)
                        && (results.len() >= 4 || query_contains_apple_music_url(query))
                    {
                        break;
                    }
                }
                Err(error) => {
                    tracing::warn!(provider = provider.name(), %error, "artwork provider failed")
                }
            }
        }

        let results = dedupe_and_sort(results, query);
        cache_artwork_results(cache_key, results.clone());
        Ok(results)
    }
}

enum ArtworkProvider {
    ITunes,
    CoverArtArchive,
}

impl ArtworkProvider {
    fn name(&self) -> &'static str {
        match self {
            Self::ITunes => "iTunes",
            Self::CoverArtArchive => "Cover Art Archive",
        }
    }

    async fn search(&self, query: &ArtworkSearchQuery) -> Result<Vec<ArtworkSearchResult>> {
        match self {
            Self::ITunes => search_itunes(query).await,
            Self::CoverArtArchive => search_cover_art_archive(query).await,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ITunesResponse {
    results: Vec<ITunesResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ITunesResult {
    wrapper_type: Option<String>,
    artist_id: Option<i64>,
    track_id: Option<i64>,
    collection_id: Option<i64>,
    track_name: Option<String>,
    collection_name: Option<String>,
    artist_name: Option<String>,
    artwork_url100: Option<String>,
}

#[derive(Deserialize)]
struct MusicBrainzRecordingResponse {
    recordings: Vec<MusicBrainzRecording>,
}

#[derive(Deserialize)]
struct MusicBrainzRecording {
    title: String,
    releases: Option<Vec<MusicBrainzRelease>>,
}

#[derive(Clone, Deserialize)]
struct MusicBrainzRelease {
    id: String,
    title: String,
}

#[derive(Deserialize)]
struct CoverArtArchiveResponse {
    images: Vec<CoverArtArchiveImage>,
}

#[derive(Deserialize)]
struct CoverArtArchiveImage {
    front: bool,
    image: String,
    thumbnails: Option<CoverArtArchiveThumbnails>,
}

#[derive(Deserialize)]
struct CoverArtArchiveThumbnails {
    small: Option<String>,
    large: Option<String>,
}

async fn search_itunes(query: &ArtworkSearchQuery) -> Result<Vec<ArtworkSearchResult>> {
    let mut results = Vec::new();
    results.extend(search_itunes_quick(query).await?);
    results.extend(search_itunes_artist_catalog(query).await?);

    Ok(results)
}

async fn search_itunes_quick(query: &ArtworkSearchQuery) -> Result<Vec<ArtworkSearchResult>> {
    let term = itunes_search_term(query);
    if term.is_empty() {
        return Ok(Vec::new());
    }

    let client = itunes_client()?;
    let mut results = Vec::new();
    if let Some(id) = apple_music_lookup_id(&term) {
        results.extend(itunes_lookup(&client, id, None, None).await?);
    }
    let (song_results, album_results) = tokio::try_join!(
        itunes_search(&client, &term, "song", "24"),
        itunes_search(&client, &term, "album", "24"),
    )?;
    results.extend(song_results);
    results.extend(album_results);

    Ok(map_itunes_results(results, query))
}

async fn search_itunes_artist_catalog(
    query: &ArtworkSearchQuery,
) -> Result<Vec<ArtworkSearchResult>> {
    let artist = query.artist.trim();
    if artist.is_empty() {
        return Ok(Vec::new());
    }

    let client = itunes_client()?;
    let artist_results = itunes_search(&client, artist, "musicArtist", "3").await?;
    let mut results = Vec::new();

    for artist_result in artist_results {
        let candidate_artist =
            normalize_text(artist_result.artist_name.as_deref().unwrap_or_default());
        let requested_artist = normalize_text(artist);
        if candidate_artist != requested_artist && !candidate_artist.contains(&requested_artist) {
            continue;
        }
        let Some(artist_id) = artist_result.artist_id else {
            continue;
        };
        let song_results = itunes_lookup(&client, artist_id, Some("song"), Some("75")).await?;
        results.extend(song_results);
        break;
    }

    Ok(map_itunes_results(results, query))
}

fn itunes_search_term(query: &ArtworkSearchQuery) -> String {
    [
        query.title.trim(),
        query.artist.trim(),
        query.album.as_deref().unwrap_or_default().trim(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

fn itunes_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?)
}

fn map_itunes_results(
    results: Vec<ITunesResult>,
    query: &ArtworkSearchQuery,
) -> Vec<ArtworkSearchResult> {
    results
        .into_iter()
        .filter(|result| result.wrapper_type.as_deref() != Some("artist"))
        .filter(|result| itunes_result_matches_query(result, query))
        .filter_map(|result| {
            let artwork_url = result.artwork_url100?;
            let thumbnail_url = artwork_url.replace("100x100bb", "400x400bb");
            let image_url = artwork_url.replace("100x100bb", "1200x1200bb");
            let id = result
                .track_id
                .or(result.collection_id)
                .map(|id| id.to_string())
                .unwrap_or_else(|| image_url.clone());

            Some(ArtworkSearchResult {
                id: format!("itunes-{id}"),
                source: "iTunes".into(),
                title: result
                    .track_name
                    .or(result.collection_name)
                    .unwrap_or_default(),
                artist: result.artist_name,
                image_url,
                thumbnail_url,
                width: Some(1200),
                height: Some(1200),
            })
        })
        .collect()
}

async fn itunes_search(
    client: &reqwest::Client,
    term: &str,
    entity: &str,
    limit: &str,
) -> Result<Vec<ITunesResult>> {
    let url = Url::parse_with_params(
        "https://itunes.apple.com/search",
        &[
            ("term", term),
            ("media", "music"),
            ("entity", entity),
            ("limit", limit),
        ],
    )?;
    let response: ITunesResponse = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response.results)
}

async fn itunes_lookup(
    client: &reqwest::Client,
    id: i64,
    entity: Option<&str>,
    limit: Option<&str>,
) -> Result<Vec<ITunesResult>> {
    let id = id.to_string();
    let mut params = vec![("id", id.as_str())];
    if let Some(entity) = entity {
        params.push(("entity", entity));
    }
    if let Some(limit) = limit {
        params.push(("limit", limit));
    }
    let url = Url::parse_with_params("https://itunes.apple.com/lookup", &params)?;
    let response: ITunesResponse = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response.results)
}

fn apple_music_lookup_id(term: &str) -> Option<i64> {
    let url = Url::parse(term).ok()?;
    if !url.domain()?.ends_with("music.apple.com") {
        return None;
    }

    url.path_segments()?
        .filter_map(|segment| segment.parse::<i64>().ok())
        .next_back()
}

fn query_contains_apple_music_url(query: &ArtworkSearchQuery) -> bool {
    apple_music_lookup_id(&itunes_search_term(query)).is_some()
}

fn itunes_result_matches_query(result: &ITunesResult, query: &ArtworkSearchQuery) -> bool {
    let title = normalized_query_title(query);
    let artist = normalize_text(&query.artist);
    if title.is_empty() {
        return true;
    }

    let result_title = normalize_text(
        result
            .track_name
            .as_deref()
            .or(result.collection_name.as_deref())
            .unwrap_or_default(),
    );
    let result_artist = normalize_text(result.artist_name.as_deref().unwrap_or_default());

    (!result_title.is_empty() && result_title.contains(&title))
        || (!result_title.is_empty() && title.contains(&result_title))
        || (!artist.is_empty() && result_artist.contains(&artist) && title.len() <= 4)
}

fn normalized_query_title(query: &ArtworkSearchQuery) -> String {
    let mut title = normalize_text(&query.title);
    let artist = normalize_text(&query.artist);
    if !artist.is_empty() {
        title = title.replace(&artist, " ");
    }
    normalize_text(&title)
}

fn normalize_text(value: &str) -> String {
    normalize_cyrillic_combining(value)
        .chars()
        .flat_map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_lowercase().collect::<Vec<_>>()
            } else {
                vec![' ']
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_cyrillic_combining(value: &str) -> String {
    value
        .replace("и\u{0306}", "й")
        .replace("И\u{0306}", "Й")
        .replace("е\u{0308}", "ё")
        .replace("Е\u{0308}", "Ё")
}

async fn search_cover_art_archive(query: &ArtworkSearchQuery) -> Result<Vec<ArtworkSearchResult>> {
    let title = query.title.trim();
    let artist = query.artist.trim();
    if title.is_empty() && artist.is_empty() {
        return Ok(Vec::new());
    }

    let mb_query = if title.is_empty() {
        format!("artist:{artist}")
    } else if artist.is_empty() {
        format!("recording:{title}")
    } else {
        format!("recording:{title} AND artist:{artist}")
    };
    let url = Url::parse_with_params(
        "https://musicbrainz.org/ws/2/recording/",
        &[
            ("query", mb_query.as_str()),
            ("fmt", "json"),
            ("inc", "releases"),
            ("limit", "8"),
        ],
    )?;
    let response: MusicBrainzRecordingResponse = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let releases = response
        .recordings
        .into_iter()
        .flat_map(|recording| {
            recording
                .releases
                .unwrap_or_default()
                .into_iter()
                .map(move |release| (recording.title.clone(), release))
        })
        .collect::<Vec<_>>();

    let mut seen_releases = HashSet::new();
    let releases = releases
        .into_iter()
        .filter(|(_, release)| seen_releases.insert(release.id.clone()))
        .take(8)
        .collect::<Vec<_>>();

    let client = reqwest::Client::new();
    let results = stream::iter(releases)
        .map(|(recording_title, release)| {
            let client = client.clone();
            async move { cover_art_for_release(client, recording_title, release).await }
        })
        .buffer_unordered(4)
        .filter_map(|result| async { result.ok().flatten() })
        .collect::<Vec<_>>()
        .await;

    Ok(results)
}

async fn cover_art_for_release(
    client: reqwest::Client,
    recording_title: String,
    release: MusicBrainzRelease,
) -> Result<Option<ArtworkSearchResult>> {
    let url = format!("https://coverartarchive.org/release/{}", release.id);
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let response: CoverArtArchiveResponse = response.error_for_status()?.json().await?;
    let mut images = response.images;
    let front_index = images.iter().position(|image| image.front);
    let image = front_index
        .map(|index| images.remove(index))
        .or_else(|| images.into_iter().next());
    let Some(image) = image else {
        return Ok(None);
    };
    let thumbnail_url = image
        .thumbnails
        .as_ref()
        .and_then(|thumbnails| {
            thumbnails
                .large
                .clone()
                .or_else(|| thumbnails.small.clone())
        })
        .unwrap_or_else(|| image.image.clone());

    Ok(Some(ArtworkSearchResult {
        id: format!("caa-{}", release.id),
        source: "Cover Art Archive".into(),
        title: if release.title.is_empty() {
            recording_title
        } else {
            release.title
        },
        artist: None,
        image_url: image.image,
        thumbnail_url,
        width: None,
        height: None,
    }))
}

fn dedupe_and_sort(
    results: Vec<ArtworkSearchResult>,
    query: &ArtworkSearchQuery,
) -> Vec<ArtworkSearchResult> {
    let mut seen = HashSet::new();
    let mut deduped = results
        .into_iter()
        .filter(|result| seen.insert(result.image_url.clone()))
        .collect::<Vec<_>>();

    deduped.sort_by_key(|result| std::cmp::Reverse(artwork_relevance(result, query)));
    deduped.truncate(18);
    deduped
}

fn artwork_relevance(result: &ArtworkSearchResult, query: &ArtworkSearchQuery) -> u32 {
    let query_title = normalized_query_title(query);
    let query_artist = normalize_text(&query.artist);
    let result_title = normalize_text(&result.title);
    let result_artist = normalize_text(result.artist.as_deref().unwrap_or_default());
    let area = result.width.unwrap_or(0) * result.height.unwrap_or(0);

    let mut score = area.min(1_440_000) / 10_000;
    if !query_title.is_empty() && result_title == query_title {
        score += 400;
    } else if !query_title.is_empty()
        && (result_title.contains(&query_title) || query_title.contains(&result_title))
    {
        score += 250;
    }
    if !query_artist.is_empty() && result_artist == query_artist {
        score += 300;
    } else if !query_artist.is_empty()
        && (result_artist.contains(&query_artist) || query_artist.contains(&result_artist))
    {
        score += 150;
    }

    score
}

fn artwork_cache() -> &'static Mutex<HashMap<String, (Instant, Vec<ArtworkSearchResult>)>> {
    ARTWORK_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn artwork_cache_key(query: &ArtworkSearchQuery) -> String {
    [
        normalize_text(&query.title),
        normalize_text(&query.artist),
        normalize_text(query.album.as_deref().unwrap_or_default()),
    ]
    .join("|")
}

fn cached_artwork_results(key: &str) -> Option<Vec<ArtworkSearchResult>> {
    let cache = artwork_cache().lock().ok()?;
    let (cached_at, results) = cache.get(key)?;
    if cached_at.elapsed() <= ARTWORK_CACHE_TTL {
        return Some(results.clone());
    }
    None
}

fn cache_artwork_results(key: String, results: Vec<ArtworkSearchResult>) {
    if let Ok(mut cache) = artwork_cache().lock() {
        cache.retain(|_, (cached_at, _)| cached_at.elapsed() <= ARTWORK_CACHE_TTL);
        cache.insert(key, (Instant::now(), results));
    }
}

fn artwork_chunk(results: Vec<ArtworkSearchResult>) -> Bytes {
    Bytes::from(format!(
        "{}\n",
        serde_json::to_string(&results).unwrap_or_default()
    ))
}
