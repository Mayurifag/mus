use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use reqwest::{Client, Request, Url};
use serde::Deserialize;
use serde_json::Value;
use tokio::process::Command;

use super::diagnostics::{youtube_video_id, yt_dlp_output};

pub(super) const OEMBED_TIMEOUT: Duration = Duration::from_secs(2);
const EXTRACTOR_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_OEMBED_BYTES: usize = 64 * 1024;

pub(super) struct SourceMetadata {
    pub title: String,
    pub artist: Option<String>,
    pub channel: String,
    pub thumbnail_url: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Deserialize)]
struct OEmbed {
    title: String,
    author_name: String,
    thumbnail_url: Option<String>,
}

pub(super) fn oembed_request(client: &Client, url: &str) -> Option<Request> {
    let id = youtube_video_id(url)?;
    client
        .get("https://www.youtube.com/oembed")
        .query(&[
            ("url", format!("https://www.youtube.com/watch?v={id}")),
            ("format", "json".into()),
        ])
        .build()
        .ok()
}

pub(super) fn metadata_command(url: &str) -> Command {
    let mut command = Command::new("yt-dlp");
    command.args([
        "--dump-json",
        "--skip-download",
        "--no-playlist",
        "--no-check-formats",
        "--ignore-no-formats-error",
        "--socket-timeout",
        "3",
        "--retries",
        "0",
        "--extractor-retries",
        "0",
        "--",
        url,
    ]);
    command
}

pub(super) fn oembed_client() -> reqwest::ClientBuilder {
    Client::builder()
        .timeout(OEMBED_TIMEOUT)
        .connect_timeout(Duration::from_secs(1))
        .redirect(reqwest::redirect::Policy::none())
}

pub(super) async fn source_metadata(url: &str) -> Result<SourceMetadata> {
    static CLIENT: OnceLock<Option<Client>> = OnceLock::new();
    let client = CLIENT.get_or_init(|| oembed_client().build().ok());
    let oembed = client
        .as_ref()
        .and_then(|client| oembed_request(client, url).map(|request| (client.clone(), request)));
    metadata_from_sources(oembed, metadata_command(url)).await
}

pub(super) async fn metadata_from_sources(
    oembed: Option<(Client, Request)>,
    fallback: Command,
) -> Result<SourceMetadata> {
    if let Some((client, request)) = oembed {
        let started = Instant::now();
        match tokio::time::timeout(OEMBED_TIMEOUT, read_oembed(&client, request)).await {
            Ok(Ok(data)) => {
                tracing::info!(
                    source = "youtube_oembed",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "download metadata ready"
                );
                return Ok(data);
            }
            _ => tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                "YouTube oEmbed unavailable; using bounded extraction"
            ),
        }
    }
    let started = Instant::now();
    let output = yt_dlp_output(fallback, EXTRACTOR_TIMEOUT).await;
    tracing::info!(
        source = "yt-dlp",
        elapsed_ms = started.elapsed().as_millis() as u64,
        success = output.is_ok(),
        "download metadata extraction finished"
    );
    let data: Value = serde_json::from_slice(&output?.stdout)
        .map_err(|_| anyhow!("yt-dlp returned invalid metadata"))?;
    let text = |key| data.get(key).and_then(Value::as_str);
    let title = text("title")
        .filter(|title| !title.trim().is_empty())
        .ok_or_else(|| anyhow!("yt-dlp returned no title"))?
        .to_string();
    Ok(SourceMetadata {
        title,
        artist: text("artist").map(str::to_string),
        channel: text("channel")
            .or_else(|| text("uploader"))
            .unwrap_or("")
            .to_string(),
        thumbnail_url: text("thumbnail").map(str::to_string),
        duration: data.get("duration").and_then(Value::as_f64),
    })
}

async fn read_oembed(client: &Client, request: Request) -> Result<SourceMetadata> {
    let mut response = client
        .execute(request)
        .await
        .map_err(|_| anyhow!("oEmbed request failed"))?;
    if !response.status().is_success() {
        return Err(anyhow!("oEmbed unavailable"));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow!("oEmbed body unavailable"))?
    {
        if body.len() + chunk.len() > MAX_OEMBED_BYTES {
            return Err(anyhow!("oEmbed response too large"));
        }
        body.extend_from_slice(&chunk);
    }
    let data: OEmbed =
        serde_json::from_slice(&body).map_err(|_| anyhow!("Invalid oEmbed metadata"))?;
    if data.title.trim().is_empty() || data.author_name.trim().is_empty() {
        return Err(anyhow!("Incomplete oEmbed metadata"));
    }
    let thumbnail_url = data.thumbnail_url.filter(|value| {
        Url::parse(value).is_ok_and(|url| {
            url.scheme() == "https"
                && url.host_str() == Some("i.ytimg.com")
                && url.username().is_empty()
                && url.password().is_none()
        })
    });
    Ok(SourceMetadata {
        title: data.title,
        artist: None,
        channel: data.author_name,
        thumbnail_url,
        duration: None,
    })
}
