use std::{collections::HashSet, path::PathBuf, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use axum::{extract::Query, Json};
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

pub async fn search_artwork(
    Query(query): Query<ArtworkSearchQuery>,
) -> Result<Json<Vec<ArtworkSearchResult>>, AppError> {
    let results = ArtworkProviders::default().search(&query).await?;
    Ok(Json(results))
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
        let mut results = Vec::new();
        for provider in &self.providers {
            match provider.search(query).await {
                Ok(mut provider_results) => results.append(&mut provider_results),
                Err(error) => {
                    tracing::warn!(provider = provider.name(), %error, "artwork provider failed")
                }
            }
        }

        Ok(dedupe_and_sort(results))
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
    let term = [
        query.title.trim(),
        query.artist.trim(),
        query.album.as_deref().unwrap_or_default().trim(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join(" ");

    if term.is_empty() {
        return Ok(Vec::new());
    }

    let url = Url::parse_with_params(
        "https://itunes.apple.com/search",
        &[
            ("term", term.as_str()),
            ("media", "music"),
            ("entity", "song"),
            ("limit", "24"),
        ],
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let response: ITunesResponse = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response
        .results
        .into_iter()
        .filter_map(|result| {
            let thumbnail_url = result.artwork_url100?;
            let image_url = thumbnail_url.replace("100x100bb", "1200x1200bb");
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
        .collect())
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

fn dedupe_and_sort(results: Vec<ArtworkSearchResult>) -> Vec<ArtworkSearchResult> {
    let mut seen = HashSet::new();
    let mut deduped = results
        .into_iter()
        .filter(|result| seen.insert(result.image_url.clone()))
        .collect::<Vec<_>>();

    deduped.sort_by_key(|result| {
        std::cmp::Reverse(result.width.unwrap_or(0) * result.height.unwrap_or(0))
    });
    deduped.truncate(18);
    deduped
}
