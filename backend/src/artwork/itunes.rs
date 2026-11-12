use std::time::Duration;

use anyhow::Result;
use reqwest::Url;
use serde::Deserialize;

use super::{normalize_text, normalized_query_title, search_term, USER_AGENT};
use crate::models::{ArtworkSearchQuery, ArtworkSearchResult};

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

pub(super) async fn search_itunes(query: &ArtworkSearchQuery) -> Result<Vec<ArtworkSearchResult>> {
    let mut results = search_itunes_quick(query).await?;
    results.extend(search_itunes_artist_catalog(query).await?);
    Ok(results)
}

pub(super) async fn search_itunes_quick(
    query: &ArtworkSearchQuery,
) -> Result<Vec<ArtworkSearchResult>> {
    let term = search_term(query);
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let client = client()?;
    let mut results = Vec::new();
    if let Some(id) = apple_music_lookup_id(&term) {
        results.extend(lookup(&client, id, None, None).await?);
    }
    let (songs, albums) = tokio::try_join!(
        search(&client, &term, "song", "24"),
        search(&client, &term, "album", "24"),
    )?;
    results.extend(songs);
    results.extend(albums);
    Ok(map_results(results, query))
}

pub(super) async fn search_itunes_artist_catalog(
    query: &ArtworkSearchQuery,
) -> Result<Vec<ArtworkSearchResult>> {
    let artist = query.artist.trim();
    if artist.is_empty() {
        return Ok(Vec::new());
    }
    let client = client()?;
    let artists = search(&client, artist, "musicArtist", "3").await?;
    let mut results = Vec::new();
    let requested_artist = normalize_text(artist);
    for artist in artists {
        let candidate = normalize_text(artist.artist_name.as_deref().unwrap_or_default());
        if candidate != requested_artist && !candidate.contains(&requested_artist) {
            continue;
        }
        let Some(id) = artist.artist_id else {
            continue;
        };
        results.extend(lookup(&client, id, Some("song"), Some("75")).await?);
        break;
    }
    Ok(map_results(results, query))
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?)
}

fn map_results(results: Vec<ITunesResult>, query: &ArtworkSearchQuery) -> Vec<ArtworkSearchResult> {
    results
        .into_iter()
        .filter(|result| result.wrapper_type.as_deref() != Some("artist"))
        .filter(|result| matches_query(result, query))
        .filter_map(|result| {
            let artwork = result.artwork_url100?;
            let thumbnail_url = artwork.replace("100x100bb", "400x400bb");
            let image_url = artwork.replace("100x100bb", "1200x1200bb");
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

async fn search(
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

async fn lookup(
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

pub(super) fn query_contains_apple_music_url(query: &ArtworkSearchQuery) -> bool {
    apple_music_lookup_id(&search_term(query)).is_some()
}

fn matches_query(result: &ITunesResult, query: &ArtworkSearchQuery) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_matching_song_artwork_without_returning_artists_or_unrelated_titles() {
        let response: ITunesResponse = serde_json::from_value(serde_json::json!({"results": [
            {"wrapperType": "artist", "artistName": "Artist", "artworkUrl100": "artist.jpg"},
            {"trackId": 42, "trackName": "Песня", "artistName": "Artist", "artworkUrl100": "https://example.com/100x100bb.jpg"},
            {"trackId": 43, "trackName": "Another song", "artworkUrl100": "other.jpg"}
        ]})).unwrap();
        let query = ArtworkSearchQuery {
            title: "Artist - Песня".into(),
            artist: "Artist".into(),
            album: None,
        };
        let results = map_results(response.results, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "itunes-42");
        assert_eq!(results[0].image_url, "https://example.com/1200x1200bb.jpg");
        assert_eq!(
            results[0].thumbnail_url,
            "https://example.com/400x400bb.jpg"
        );
    }
}
