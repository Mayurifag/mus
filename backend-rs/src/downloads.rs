use std::{fs, process::Stdio};

use anyhow::{anyhow, Result};
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use tokio::process::Command;

use crate::{
    error::AppError,
    events::broadcast,
    media::write_audio_tags,
    models::{ConfirmDownloadRequest, MetadataResponse, UrlRequest},
    scanner::upsert_path,
    state::AppState,
    util::{
        audio_paths, can_write, clean_title, command_output, extract_artist_title,
        generate_filename, now,
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
    let output = command_output("yt-dlp", &["--dump-json", "--no-playlist", &req.url]).await?;
    let data: Value = serde_json::from_str(&output)?;
    let raw_title = data
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Unknown Title");
    let raw_artist = data.get("artist").and_then(Value::as_str);
    let channel = data
        .get("channel")
        .or_else(|| data.get("uploader"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let (artist, title) = raw_artist
        .map(|artist| (artist.to_string(), clean_title(raw_title)))
        .unwrap_or_else(|| extract_artist_title(raw_title, channel));
    Ok(Json(MetadataResponse {
        title,
        artist,
        thumbnail_url: data
            .get("thumbnail")
            .and_then(Value::as_str)
            .map(str::to_string),
        duration: data.get("duration").and_then(Value::as_f64),
    }))
}

pub async fn start_download(
    State(state): State<AppState>,
    Json(req): Json<UrlRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    spawn_download(state, req.url, None, None).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"status": "accepted"}))))
}

pub async fn confirm_download(
    State(state): State<AppState>,
    Json(req): Json<ConfirmDownloadRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let final_name = generate_filename(&req.artist, &req.title, "mp3")?;
    if state.music_dir.join(final_name).exists() {
        return Err(AppError::conflict("A file with this name already exists"));
    }
    spawn_download(state, req.url, Some(req.title), Some(req.artist)).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"status": "accepted"}))))
}

async fn spawn_download(
    state: AppState,
    url: String,
    title: Option<String>,
    artist: Option<String>,
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
    tokio::spawn(async move {
        broadcast(
            &state,
            "download_started",
            Some("Download started"),
            Some("info"),
            Some(json!({"url": url})),
        );
        let result = run_download(state.clone(), url, title, artist).await;
        if let Err(error) = result {
            tracing::warn!(error = %error, "download failed");
            broadcast(
                &state,
                "download_failed",
                Some(&format!("Download failed: {error}")),
                Some("error"),
                Some(json!({"error": error.to_string()})),
            );
        }
        *state.download_lock.lock().unwrap() = false;
    });
    Ok(())
}

async fn run_download(
    state: AppState,
    url: String,
    title: Option<String>,
    artist: Option<String>,
) -> Result<()> {
    let tmp = state
        .data_dir
        .join(".cache")
        .join(format!("download-{}", now()));
    fs::create_dir_all(&tmp)?;
    let result = run_download_inner(&state, &url, title, artist, &tmp).await;
    let _ = fs::remove_dir_all(tmp);
    result
}

async fn run_download_inner(
    state: &AppState,
    url: &str,
    title: Option<String>,
    artist: Option<String>,
    tmp: &std::path::Path,
) -> Result<()> {
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
    .arg(output_template)
    .args([
        "--embed-thumbnail",
        "--convert-thumbnails",
        "jpg",
        "--embed-metadata",
        "--no-playlist",
        url,
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!(String::from_utf8_lossy(&output.stderr).to_string()));
    }
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
    if final_path.exists() {
        return Err(anyhow!("A file with this name already exists"));
    }
    fs::rename(&downloaded, &final_path)?;
    if let (Some(title), Some(artist)) = (&title, &artist) {
        write_audio_tags(&final_path, title, artist).await?;
    }
    let track = upsert_path(state, &final_path, title, artist).await?;
    tracing::info!(track_id = track.id, "download completed");
    broadcast(
        state,
        "download_completed",
        Some("Download completed successfully"),
        Some("success"),
        Some(json!({"file_path": final_path})),
    );
    Ok(())
}
