use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;

use super::search_term;
use crate::{
    models::{ArtworkSearchQuery, ArtworkSearchResult},
    process::command_output_text_with_timeout,
};

#[derive(Deserialize)]
struct YouTubeSearchResult {
    title: Option<String>,
    uploader: Option<String>,
    thumbnail: Option<String>,
    id: Option<String>,
}

pub(super) async fn search_youtube(query: &ArtworkSearchQuery) -> Result<Vec<ArtworkSearchResult>> {
    let term = search_term(query);
    if term.is_empty() {
        return Ok(Vec::new());
    }
    let search = format!("ytsearch3:{term}");
    let output = command_output_text_with_timeout(
        "yt-dlp",
        &["--dump-json", "--skip-download", &search],
        Duration::from_secs(20),
    )
    .await?;
    Ok(output
        .lines()
        .filter_map(|line| serde_json::from_str::<YouTubeSearchResult>(line).ok())
        .filter_map(|result| {
            let image_url = result.thumbnail?;
            let id = result.id.unwrap_or_else(|| image_url.clone());
            Some(ArtworkSearchResult {
                id: format!("youtube-{id}"),
                source: "YouTube".into(),
                title: result.title.unwrap_or_default(),
                artist: result.uploader,
                thumbnail_url: image_url.clone(),
                image_url,
                width: None,
                height: None,
            })
        })
        .collect())
}
