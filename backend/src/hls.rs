use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use axum::{
    extract::{Path as AxumPath, Request, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{db::get_track_path, error::AppError, models::Track, state::AppState};

mod cache;
use cache::{ensure_hls_cache, hls_cache_dir};

pub fn track_hls_url(track: &Track) -> String {
    format!("/api/v1/tracks/{}/hls/index.m3u8", track.id)
}

pub async fn get_hls_playlist(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    request: Request,
) -> Result<Response, AppError> {
    let source = get_track_path(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    if !tokio::fs::metadata(&source)
        .await
        .is_ok_and(|meta| meta.is_file())
    {
        return Err(AppError::not_found("Audio file not found"));
    }
    let playlist = ensure_hls_cache(&state, &id, &source)
        .await?
        .join("index.m3u8");
    if !tokio::fs::metadata(&playlist)
        .await
        .is_ok_and(|meta| meta.is_file())
    {
        return Err(AppError::not_found("HLS playlist not found"));
    }
    Ok(serve_hls_file(
        playlist,
        request,
        "application/vnd.apple.mpegurl",
        "no-cache",
    )
    .await)
}

pub async fn get_hls_segment(
    State(state): State<AppState>,
    AxumPath((id, segment)): AxumPath<(String, String)>,
    request: Request,
) -> Result<Response, AppError> {
    if segment != "init.mp4" && !is_hls_segment(&segment) {
        return Err(AppError::not_found("HLS segment not found"));
    }
    let source = get_track_path(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    let file_path = hls_cache_dir(&state, &id).join(segment);
    let file_path = tokio::task::spawn_blocking(move || {
        require_file(&source, "Audio file not found")?;
        require_file(&file_path, "HLS segment not found")?;
        Ok::<_, AppError>(file_path)
    })
    .await
    .context("HLS file lookup failed")??;
    Ok(serve_hls_file(
        file_path,
        request,
        "audio/mp4",
        "public, max-age=31536000, immutable",
    )
    .await)
}

fn require_file(path: &Path, detail: &str) -> Result<(), AppError> {
    if fs::metadata(path).is_ok_and(|meta| meta.is_file()) {
        Ok(())
    } else {
        Err(AppError::not_found(detail))
    }
}

async fn serve_hls_file(
    file_path: PathBuf,
    request: Request,
    content_type: &'static str,
    cache_control: &'static str,
) -> Response {
    let Ok(response) = ServeFile::new(file_path).oneshot(request).await;
    let mut response = response.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    response
}

fn is_hls_segment(value: &str) -> bool {
    let Some(number) = value
        .strip_prefix("segment-")
        .and_then(|value| value.strip_suffix(".m4s"))
    else {
        return false;
    };
    !number.is_empty() && number.len() <= 16 && number.bytes().all(|byte| byte.is_ascii_digit())
}
