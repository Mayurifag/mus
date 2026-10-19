use std::{
    fs,
    future::Future,
    io,
    path::Path,
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use tokio::process::Command;
use tracing::Instrument;

mod diagnostics;
mod info;
#[cfg(test)]
mod latency_tests;
mod publish;
#[cfg(test)]
mod tests;

use diagnostics::{log_versions, safe_failure, yt_dlp_output, DownloadJob};
use info::{source_metadata, SourceMetadata};

use crate::{
    artwork::download_artwork_as_jpeg,
    db,
    error::AppError,
    events::broadcast,
    metadata::{parse_track_metadata, TrackMetadata},
    models::{ConfirmDownloadRequest, MetadataResponse, UrlRequest},
    scanner::upsert_path,
    state::AppState,
    util::{
        audio_paths, can_write, clean_title, extract_artist_title, generate_filename,
        inferred_tag_names, normalize_artists, normalize_text, now_nanos,
    },
};

pub async fn fetch_metadata(
    State(state): State<AppState>,
    Json(req): Json<UrlRequest>,
) -> Result<Json<MetadataResponse>, AppError> {
    if !can_write(&state.music_dir) {
        return Err(AppError::forbidden(
            "Download not available - insufficient write permissions to music directory",
        ));
    }
    let job = DownloadJob::new(&req.url);
    async {
        let started = Instant::now();
        let data = source_metadata(&req.url).await?;
        let response =
            metadata_response(&data, parse_track_metadata(&data.title, &data.channel)).await;
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            "download name form ready"
        );
        Ok(Json(response))
    }
    .instrument(job.span("metadata"))
    .await
}

async fn metadata_response(
    data: &SourceMetadata,
    ai: impl Future<Output = Option<TrackMetadata>>,
) -> MetadataResponse {
    let started = Instant::now();
    let parsed = tokio::time::timeout(Duration::from_secs(2), ai).await;
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        timed_out = parsed.is_err(),
        "download name enrichment finished"
    );
    let (artist, title, mut tag_names) = match parsed.ok().flatten() {
        Some(metadata) => (metadata.artist, metadata.title, metadata.tags),
        None => data
            .artist
            .as_deref()
            .map(|artist| (artist.to_string(), clean_title(&data.title), Vec::new()))
            .unwrap_or_else(|| {
                let (artist, title) = extract_artist_title(&data.title, &data.channel);
                (artist, title, Vec::new())
            }),
    };
    let title = normalize_text(&title);
    let artist = normalize_artists(&artist);
    for tag_name in inferred_tag_names(&title, &artist, Path::new(&data.title)) {
        if !tag_names.contains(&tag_name) {
            tag_names.push(tag_name);
        }
    }
    MetadataResponse {
        title,
        artist,
        thumbnail_url: data.thumbnail_url.clone(),
        duration: data.duration,
        tags: tag_names
            .into_iter()
            .map(|name| crate::models::Tag {
                display_name: match name.as_str() {
                    "right-version" => "Right version",
                    "ai-cover" => "AI cover",
                    "cover" => "Cover",
                    "ost" => "OST",
                    "mashup" => "Mashup",
                    _ => name.as_str(),
                }
                .into(),
                name,
            })
            .collect(),
    }
}

pub async fn start_download(
    State(state): State<AppState>,
    Json(req): Json<UrlRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    spawn_download(state, req.url, None, None, None, Vec::new()).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"status": "accepted"}))))
}

pub async fn confirm_download(
    State(state): State<AppState>,
    Json(req): Json<ConfirmDownloadRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let artist = normalize_artists(&req.artist);
    let title = normalize_text(&req.title);
    let final_name = generate_filename(&artist, &title, "mp3")?;
    if state.music_dir.join(final_name).exists() {
        return Err(AppError::conflict("A file with this name already exists"));
    }
    spawn_download(
        state,
        req.url,
        Some(title),
        Some(artist),
        req.artwork_url.filter(|url| !url.trim().is_empty()),
        req.tags.unwrap_or_default(),
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"status": "accepted"}))))
}

async fn spawn_download(
    state: AppState,
    url: String,
    title: Option<String>,
    artist: Option<String>,
    artwork_url: Option<String>,
    tags: Vec<String>,
) -> Result<(), AppError> {
    if !can_write(&state.music_dir) {
        return Err(AppError::forbidden(
            "Download not available - insufficient write permissions to music directory",
        ));
    }
    {
        let mut locked = state.download_lock.lock().unwrap();
        if *locked {
            return Err(AppError::too_many("Download already in progress"));
        }
        *locked = true;
    }
    let job = DownloadJob::new(&url);
    let span = job.span("download");
    tokio::spawn(
        async move {
            broadcast(
                &state,
                "download_started",
                Some("Download started"),
                Some("info"),
                Some(job.payload()),
            );
            let started = Instant::now();
            tokio::spawn(log_versions().instrument(tracing::Span::current()));
            let result = run_download(state.clone(), url, title, artist, artwork_url, tags).await;
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                success = result.is_ok(),
                "download job finished"
            );
            let mut payload = job.payload();
            match result {
                Ok(final_path) => {
                    payload["file_path"] = json!(final_path);
                    broadcast(
                        &state,
                        "download_completed",
                        Some("Download completed successfully"),
                        Some("success"),
                        Some(payload),
                    );
                }
                Err(error) => {
                    let message = safe_failure(&error.to_string());
                    tracing::warn!(reason = message, "download failed");
                    payload["error"] = json!(message);
                    broadcast(
                        &state,
                        "download_failed",
                        Some(message),
                        Some("error"),
                        Some(payload),
                    );
                }
            }
            *state.download_lock.lock().unwrap() = false;
        }
        .instrument(span),
    );
    Ok(())
}

async fn run_download(
    state: AppState,
    url: String,
    title: Option<String>,
    artist: Option<String>,
    artwork_url: Option<String>,
    tags: Vec<String>,
) -> Result<PathBuf> {
    let cache_dir = state.data_dir.join(".cache");
    fs::create_dir_all(&cache_dir)?;
    let tmp = unique_download_dir(&cache_dir)?;
    let command = download_command(&url, &tmp, artwork_url.is_none());
    let result = run_download_inner(&state, command, title, artist, artwork_url, tags, &tmp).await;
    let _ = fs::remove_dir_all(tmp);
    result
}

async fn run_download_inner(
    state: &AppState,
    command: Command,
    title: Option<String>,
    artist: Option<String>,
    artwork_url: Option<String>,
    tags: Vec<String>,
    tmp: &std::path::Path,
) -> Result<PathBuf> {
    let title = title.map(|title| normalize_text(&title));
    let artist = artist.map(|artist| normalize_artists(&artist));
    let media = async {
        let started = Instant::now();
        let result = yt_dlp_output(command, Duration::from_secs(600)).await;
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            success = result.is_ok(),
            "download media processing finished"
        );
        result
    };
    let artwork = async {
        let started = Instant::now();
        let result = match artwork_url {
            Some(url) => download_artwork_as_jpeg(&url, tmp.to_path_buf())
                .await
                .map(Some),
            None => Ok(None),
        };
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            success = result.is_ok(),
            "download artwork processing finished"
        );
        result
    };
    // Do not drop a subprocess-owning future when the other operation fails.
    let (media_result, artwork_result) = tokio::join!(media, artwork);
    media_result?;
    let jpeg_path = artwork_result?;
    let downloaded = audio_paths(tmp)?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Downloaded file not found"))?;
    let final_name = match (&title, &artist) {
        (Some(title), Some(artist)) => {
            generate_filename(artist, title, "mp3").map_err(|error| anyhow!(error.message))?
        }
        _ => downloaded
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("download.mp3")
            .to_string(),
    };
    let final_path = state.music_dir.join(final_name);
    let started = Instant::now();
    publish::publish_download(
        &downloaded,
        tmp,
        &final_path,
        title.as_deref(),
        artist.as_deref(),
        jpeg_path.as_deref(),
    )
    .await?;
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        "download tags and publication finished"
    );
    let started = Instant::now();
    let track = upsert_path(state, &final_path, title, artist).await?;
    db::add_track_tags(state, &track.id, &tags)?;
    tracing::info!(
        track_id = track.id,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "download library registration and HLS prewarm finished"
    );
    Ok(final_path)
}

fn download_command(url: &str, tmp: &Path, embed_thumbnail: bool) -> Command {
    let output_template = tmp.join("%(artist,uploader|Unknown Artist)s - %(title)s.%(ext)s");
    let mut cmd = Command::new("yt-dlp");
    cmd.args([
        "--format",
        "bestaudio/best",
        "--extract-audio",
        "--audio-format",
        "mp3",
        "--audio-quality",
        "0",
        "-o",
    ])
    .arg(output_template);
    if embed_thumbnail {
        cmd.args(["--embed-thumbnail", "--convert-thumbnails", "jpg"]);
    }
    cmd.args([
        "--embed-metadata",
        "--parse-metadata",
        "title:%(title)s",
        "--parse-metadata",
        "artist:%(artist,uploader|Unknown Artist)s",
        "--sponsorblock-remove",
        "all",
        "--embed-chapters",
        "--concurrent-fragments",
        "3",
        "--throttled-rate",
        "100K",
        "--retries",
        "10",
        "--no-playlist",
        "--",
        url,
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    cmd
}

fn unique_download_dir(cache_dir: &Path) -> Result<PathBuf> {
    for attempt in 0..100 {
        let path = cache_dir.join(format!("download-{}-{attempt}", now_nanos()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(anyhow!("failed to allocate download directory"))
}
