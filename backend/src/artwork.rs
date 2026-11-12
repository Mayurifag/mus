use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use anyhow::anyhow;
use axum::{
    body::{Body, Bytes},
    extract::Query,
    http::header,
    response::Response,
    Json,
};

use crate::{
    error::AppError,
    models::{ArtworkSearchQuery, ArtworkSearchResult},
};

mod cover_art_archive;
mod download;
use cover_art_archive::search_cover_art_archive;
mod itunes;
mod youtube;
pub use download::download_artwork_as_jpeg;
use itunes::{
    query_contains_apple_music_url, search_itunes, search_itunes_artist_catalog,
    search_itunes_quick,
};
use youtube::search_youtube;

const USER_AGENT: &str = "mus/0.1 artwork search";
const ARTWORK_CACHE_TTL: Duration = Duration::from_secs(600);
type ArtworkCacheEntry = (Instant, Vec<ArtworkSearchResult>);
type ArtworkCache = Mutex<HashMap<String, ArtworkCacheEntry>>;
static ARTWORK_CACHE: OnceLock<ArtworkCache> = OnceLock::new();

pub async fn search_artwork(
    Query(query): Query<ArtworkSearchQuery>,
) -> Result<Json<Vec<ArtworkSearchResult>>, AppError> {
    let results = search(&query).await;
    Ok(Json(results))
}

pub async fn stream_artwork(Query(query): Query<ArtworkSearchQuery>) -> Result<Response, AppError> {
    let stream = async_stream::stream! {
        let cache_key = artwork_cache_key(&query);
        if let Some(results) = cached_artwork_results(&cache_key) {
            yield artwork_chunk(&results);
            return;
        }

        let mut results = Vec::new();
        match search_itunes_quick(&query).await {
            Ok(mut quick_results) => {
                results.append(&mut quick_results);
                let sorted = dedupe_and_sort(results.clone(), &query);
                if !sorted.is_empty() {
                    yield artwork_chunk(&sorted);
                }
            }
            Err(error) => tracing::warn!(provider = "iTunes", %error, "artwork provider failed"),
        }

        match search_itunes_artist_catalog(&query).await {
            Ok(mut artist_results) => {
                results.append(&mut artist_results);
                let sorted = dedupe_and_sort(results.clone(), &query);
                if !sorted.is_empty() {
                    yield artwork_chunk(&sorted);
                }
            }
            Err(error) => tracing::warn!(provider = "iTunes", %error, "artwork provider failed"),
        }

        if results.len() < 4 {
            match search_youtube(&query).await {
                Ok(mut youtube_results) => {
                    results.append(&mut youtube_results);
                    let sorted = dedupe_and_sort(results.clone(), &query);
                    if !sorted.is_empty() {
                        yield artwork_chunk(&sorted);
                    }
                }
                Err(error) => tracing::warn!(provider = "YouTube", %error, "artwork provider failed"),
            }
        }

        if results.len() < 4 && !query_contains_apple_music_url(&query) {
            match search_cover_art_archive(&query).await {
                Ok(mut caa_results) => {
                    results.append(&mut caa_results);
                    let sorted = dedupe_and_sort(results.clone(), &query);
                    if !sorted.is_empty() {
                        yield artwork_chunk(&sorted);
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

async fn search(query: &ArtworkSearchQuery) -> Vec<ArtworkSearchResult> {
    let cache_key = artwork_cache_key(query);
    if let Some(results) = cached_artwork_results(&cache_key) {
        return results;
    }
    let mut results = Vec::new();
    let itunes_sufficient = match search_itunes(query).await {
        Ok(itunes) => {
            results.extend(itunes);
            results.len() >= 4 || query_contains_apple_music_url(query)
        }
        Err(error) => {
            tracing::warn!(provider = "iTunes", %error, "artwork provider failed");
            false
        }
    };
    if !itunes_sufficient {
        match search_youtube(query).await {
            Ok(youtube) => results.extend(youtube),
            Err(error) => tracing::warn!(provider = "YouTube", %error, "artwork provider failed"),
        }
        match search_cover_art_archive(query).await {
            Ok(covers) => results.extend(covers),
            Err(error) => {
                tracing::warn!(provider = "Cover Art Archive", %error, "artwork provider failed")
            }
        }
    }
    let results = dedupe_and_sort(results, query);
    cache_artwork_results(cache_key, results.clone());
    results
}

fn search_term(query: &ArtworkSearchQuery) -> String {
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
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .flat_map(char::to_lowercase)
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
    let area = result
        .width
        .unwrap_or(0)
        .saturating_mul(result.height.unwrap_or(0));

    let mut score = area.min(1_440_000) / 10_000;
    if !query_title.is_empty() && result_title == query_title {
        score += 400;
    } else if !query_title.is_empty()
        && !result_title.is_empty()
        && (result_title.contains(&query_title) || query_title.contains(&result_title))
    {
        score += 250;
    }
    if !query_artist.is_empty() && result_artist == query_artist {
        score += 300;
    } else if !query_artist.is_empty()
        && !result_artist.is_empty()
        && (result_artist.contains(&query_artist) || query_artist.contains(&result_artist))
    {
        score += 150;
    }

    score
}

fn artwork_cache() -> &'static ArtworkCache {
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

fn artwork_chunk(results: &[ArtworkSearchResult]) -> serde_json::Result<Bytes> {
    let mut chunk = serde_json::to_vec(results)?;
    chunk.push(b'\n');
    Ok(Bytes::from(chunk))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artwork() -> ArtworkSearchResult {
        ArtworkSearchResult {
            id: "artwork".into(),
            source: "test".into(),
            title: "Песня\nSong".into(),
            artist: Some("Artist".into()),
            image_url: "https://example.com/cover.jpg".into(),
            thumbnail_url: "https://example.com/thumb.jpg".into(),
            width: Some(1200),
            height: Some(1200),
        }
    }

    #[test]
    fn search_normalization_preserves_unicode_lowercase_expansions() {
        assert_eq!(
            normalize_text("И\u{0306} Е\u{0308}, İ A/B\nC!"),
            "й ё i\u{0307} a b c"
        );
    }

    #[test]
    fn oversized_provider_dimensions_keep_the_capped_resolution_score() {
        let mut result = artwork();
        let query = ArtworkSearchQuery {
            title: result.title.clone(),
            artist: "Artist".into(),
            album: None,
        };
        assert_eq!(artwork_relevance(&result, &query), 844);
        result.width = Some(u32::MAX);
        result.height = Some(u32::MAX);
        assert_eq!(artwork_relevance(&result, &query), 844);
        result.height = Some(0);
        assert_eq!(artwork_relevance(&result, &query), 700);
    }

    #[test]
    fn missing_provider_text_does_not_count_as_a_partial_match() {
        let query = ArtworkSearchQuery {
            title: "Song".into(),
            artist: "Artist".into(),
            album: None,
        };
        let mut result = artwork();
        result.title = "Unrelated song title".into();
        result.artist = None;
        result.width = None;
        result.height = None;
        // Cover Art Archive has no artist field; that must not earn artist points.
        assert_eq!(artwork_relevance(&result, &query), 250);
        result.title.clear();
        assert_eq!(artwork_relevance(&result, &query), 0);
    }

    #[test]
    fn artwork_chunks_encode_one_json_array_per_line() {
        let result = artwork();
        let expected = serde_json::to_value([&result]).unwrap();
        let chunk = artwork_chunk(&[result]).unwrap();
        assert_eq!(chunk.iter().filter(|byte| **byte == b'\n').count(), 1);
        assert_eq!(chunk.last(), Some(&b'\n'));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&chunk).unwrap(),
            expected
        );
    }
}
