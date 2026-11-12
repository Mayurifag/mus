use std::collections::HashSet;

use anyhow::Result;
use futures_util::{stream, StreamExt};
use reqwest::Url;
use serde::Deserialize;

use super::USER_AGENT;
use crate::models::{ArtworkSearchQuery, ArtworkSearchResult};

#[derive(Deserialize)]
struct RecordingResponse {
    recordings: Vec<Recording>,
}
#[derive(Deserialize)]
struct Recording {
    title: String,
    releases: Option<Vec<Release>>,
}
#[derive(Deserialize)]
struct Release {
    id: String,
    title: String,
}
#[derive(Deserialize)]
struct ArtworkResponse {
    images: Vec<Image>,
}
#[derive(Deserialize)]
struct Image {
    front: bool,
    image: String,
    thumbnails: Option<Thumbnails>,
}
#[derive(Deserialize)]
struct Thumbnails {
    small: Option<String>,
    large: Option<String>,
}

pub(super) async fn search_cover_art_archive(
    query: &ArtworkSearchQuery,
) -> Result<Vec<ArtworkSearchResult>> {
    let title = query.title.trim();
    let artist = query.artist.trim();
    if title.is_empty() && artist.is_empty() {
        return Ok(Vec::new());
    }
    let query = if title.is_empty() {
        format!("artist:{artist}")
    } else if artist.is_empty() {
        format!("recording:{title}")
    } else {
        format!("recording:{title} AND artist:{artist}")
    };
    let url = Url::parse_with_params(
        "https://musicbrainz.org/ws/2/recording/",
        &[
            ("query", query.as_str()),
            ("fmt", "json"),
            ("inc", "releases"),
            ("limit", "8"),
        ],
    )?;
    let client = reqwest::Client::new();
    let response: RecordingResponse = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut seen = HashSet::new();
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
        .filter(|(_, release)| seen.insert(release.id.clone()))
        .take(8);
    Ok(stream::iter(releases)
        .map(|(title, release)| {
            let client = &client;
            async move { cover_art_for_release(client, title, release).await }
        })
        .buffer_unordered(4)
        .filter_map(|result| async { result.ok().flatten() })
        .collect()
        .await)
}

async fn cover_art_for_release(
    client: &reqwest::Client,
    title: String,
    release: Release,
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
    let response: ArtworkResponse = response.error_for_status()?.json().await?;
    Ok(release_artwork(title, release, response.images))
}

fn release_artwork(
    title: String,
    release: Release,
    mut images: Vec<Image>,
) -> Option<ArtworkSearchResult> {
    let image = match images.iter().position(|image| image.front) {
        Some(index) => images.remove(index),
        None => images.into_iter().next()?,
    };
    let thumbnail_url = image
        .thumbnails
        .and_then(|thumb| thumb.large.or(thumb.small))
        .unwrap_or_else(|| image.image.clone());
    Some(ArtworkSearchResult {
        id: format!("caa-{}", release.id),
        source: "Cover Art Archive".into(),
        title: if release.title.is_empty() {
            title
        } else {
            release.title
        },
        artist: None,
        image_url: image.image,
        thumbnail_url,
        width: None,
        height: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_front_cover_and_prefers_the_large_thumbnail() {
        let response: ArtworkResponse = serde_json::from_value(serde_json::json!({"images": [
            {"front": false, "image": "back.jpg"},
            {"front": true, "image": "front.jpg", "thumbnails": {"small": "small.jpg", "large": "large.jpg"}}
        ]})).unwrap();
        let artwork = release_artwork(
            "Recording".into(),
            Release {
                id: "release-id".into(),
                title: "Album".into(),
            },
            response.images,
        )
        .unwrap();
        assert_eq!(artwork.title, "Album");
        assert_eq!(artwork.image_url, "front.jpg");
        assert_eq!(artwork.thumbnail_url, "large.jpg");
    }

    #[test]
    fn falls_back_to_first_image_and_recording_title() {
        let images = vec![Image {
            front: false,
            image: "cover.jpg".into(),
            thumbnails: None,
        }];
        let artwork = release_artwork(
            "Recording".into(),
            Release {
                id: "id".into(),
                title: String::new(),
            },
            images,
        )
        .unwrap();
        assert_eq!(artwork.title, "Recording");
        assert_eq!(artwork.thumbnail_url, "cover.jpg");
        assert!(release_artwork(
            "Recording".into(),
            Release {
                id: "id".into(),
                title: String::new()
            },
            Vec::new()
        )
        .is_none());
    }
}
