use std::path::PathBuf;

use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Request, State},
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
};
use tokio::fs;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{db::get_track, error::AppError, models::Track, state::AppState};

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
    let track = get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    if !fs::metadata(&track.file_path)
        .await
        .is_ok_and(|meta| meta.is_file())
    {
        return Err(AppError::not_found("Audio file not found"));
    }
    let dir = ensure_hls_cache(&state, &track).await?;
    serve_hls_file(
        dir.join("index.m3u8"),
        request,
        "application/vnd.apple.mpegurl",
        "no-cache",
        "HLS playlist not found",
    )
    .await
}

pub async fn get_hls_segment(
    State(state): State<AppState>,
    AxumPath((id, segment)): AxumPath<(String, String)>,
    request: Request,
) -> Result<Response, AppError> {
    if !is_safe_hls_media_file(&segment) {
        return Err(AppError::not_found("HLS segment not found"));
    }
    let track = get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    if !fs::metadata(&track.file_path)
        .await
        .is_ok_and(|meta| meta.is_file())
    {
        return Err(AppError::not_found("Audio file not found"));
    }
    serve_hls_file(
        hls_cache_dir(&state, &id).join(segment),
        request,
        "audio/mp4",
        "public, max-age=31536000, immutable",
        "HLS segment not found",
    )
    .await
}

async fn serve_hls_file(
    file_path: PathBuf,
    request: Request,
    content_type: &'static str,
    cache_control: &'static str,
    not_found: &'static str,
) -> Result<Response, AppError> {
    if !fs::metadata(&file_path)
        .await
        .is_ok_and(|meta| meta.is_file())
    {
        return Err(AppError::not_found(not_found));
    }

    let Ok(response) = ServeFile::new(file_path).oneshot(request).await;
    let mut response = response.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    Ok(response)
}

fn is_safe_hls_media_file(value: &str) -> bool {
    if value == "init.mp4" {
        return true;
    }
    let Some(number) = value
        .strip_prefix("segment-")
        .and_then(|value| value.strip_suffix(".m4s"))
    else {
        return false;
    };
    !number.is_empty() && number.len() <= 16 && number.bytes().all(|byte| byte.is_ascii_digit())
}
