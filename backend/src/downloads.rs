use std::time::Instant;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use tokio::fs;
use tracing::Instrument;

mod diagnostics;
mod info;
#[cfg(test)]
mod latency_tests;
mod parser;
mod publish;
#[cfg(test)]
mod tests;
mod worker;

use diagnostics::{safe_failure, DownloadJob};
use info::{source_metadata, SourceMetadata};
use worker::{run_download, DownloadNaming, DownloadOptions};

use crate::{
    audio_names::generate_filename,
    categories::category_display_name,
    db,
    error::AppError,
    events::broadcast,
    files::can_write,
    models::{ConfirmDownloadRequest, MetadataResponse, UrlRequest},
    state::AppState,
};

pub async fn fetch_metadata(
    State(state): State<AppState>,
    Json(req): Json<UrlRequest>,
) -> Result<Json<MetadataResponse>, AppError> {
    if !can_write(&state.music_dir).await {
        return Err(AppError::forbidden(
            "Download not available - insufficient write permissions to music directory",
        ));
    }
    let job = DownloadJob::new(&req.url);
    async {
        let started = Instant::now();
        let data = source_metadata(&req.url).await?;
        let response = metadata_response(&data, &db::artist_values(&state).await?);
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis(),
            "download name form ready"
        );
        Ok(Json(response))
    }
    .instrument(job.span("metadata"))
    .await
}

fn metadata_response(data: &SourceMetadata, library_artists: &[String]) -> MetadataResponse {
    let parsed = parser::with_library_artists(
        parser::parse(&data.title, &data.channel, data.artist.as_deref()),
        library_artists,
    );
    MetadataResponse {
        tags: parsed
            .tags
            .into_iter()
            .map(|name| crate::models::Tag {
                display_name: category_display_name(name).unwrap_or(name).into(),
                name: name.into(),
            })
            .collect(),
        title: parsed.title,
        artist: parsed.artist,
        thumbnail_url: data.thumbnail_url.clone(),
        duration: data.duration,
    }
}

pub async fn start_download(
    State(state): State<AppState>,
    Json(req): Json<UrlRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    spawn_download(state, req.url, DownloadOptions::default()).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"status": "accepted"}))))
}

pub async fn confirm_download(
    State(state): State<AppState>,
    Json(req): Json<ConfirmDownloadRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let final_name = generate_filename(&req.artist, &req.title, "mp3")?;
    if fs::try_exists(state.music_dir.join(final_name)).await? {
        return Err(AppError::conflict("A file with this name already exists"));
    }
    spawn_download(
        state,
        req.url,
        DownloadOptions {
            naming: DownloadNaming::Custom {
                title: req.title,
                artist: req.artist,
            },
            artwork_url: req.artwork_url.filter(|url| !url.trim().is_empty()),
            tags: req.tags,
        },
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"status": "accepted"}))))
}

async fn spawn_download(
    state: AppState,
    url: String,
    options: DownloadOptions,
) -> Result<(), AppError> {
    if !can_write(&state.music_dir).await {
        return Err(AppError::forbidden(
            "Download not available - insufficient write permissions to music directory",
        ));
    }
    let permit = state
        .download_lock
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::too_many("Download already in progress"))?;
    let job = DownloadJob::new(&url);
    let span = job.span("download");
    tokio::spawn(
        async move {
            let _permit = permit;
            broadcast(
                &state,
                "download_started",
                Some("Download started"),
                Some("info"),
                Some(job.payload()),
            );
            let started = Instant::now();
            let yt_dlp_version = state.yt_dlp_version.lock().await.clone();
            tracing::info!(
                yt_dlp_version = yt_dlp_version.as_deref().unwrap_or("unknown"),
                "download started"
            );
            let result = run_download(&state, url, options).await;
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis(),
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
        }
        .instrument(span),
    );
    Ok(())
}
