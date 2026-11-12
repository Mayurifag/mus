use std::path::PathBuf;

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, HeaderValue, Request},
    response::{IntoResponse, Response},
};
use tokio::fs;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{db::get_track, error::AppError, models::Track, state::AppState};

mod cache;
pub(crate) use cache::prewarm_track_hls;
use cache::{ensure_hls_cache, hls_cache_dir};

pub fn track_hls_url(track: &Track) -> String {
    format!(
        "/api/v1/tracks/{}/hls/{}/index.m3u8",
        track.id, track.content_hash
    )
}

pub async fn get_hls_playlist(
    State(state): State<AppState>,
    AxumPath((id, cache_key)): AxumPath<(String, String)>,
) -> Result<Response, AppError> {
    if !is_safe_hls_token(&cache_key) {
        return Err(AppError::not_found("HLS playlist not found"));
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
    if cache_key != track.content_hash {
        return Err(AppError::not_found("HLS playlist not found"));
    }

    let dir = ensure_hls_cache(&state, &track, &cache_key).await?;
    serve_hls_file(
        dir.join("index.m3u8"),
        "application/vnd.apple.mpegurl",
        "no-cache",
        "HLS playlist not found",
    )
    .await
}

pub async fn get_hls_segment(
    State(state): State<AppState>,
    AxumPath((id, cache_key, segment)): AxumPath<(String, String, String)>,
) -> Result<Response, AppError> {
    if !is_safe_hls_token(&cache_key) || !is_safe_hls_media_file(&segment) {
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
    if cache_key != track.content_hash {
        return Err(AppError::not_found("HLS segment not found"));
    }

    serve_hls_file(
        hls_cache_dir(&state, &id, &cache_key).join(segment),
        "audio/mp4",
        "public, max-age=31536000, immutable",
        "HLS segment not found",
    )
    .await
}

async fn serve_hls_file(
    file_path: PathBuf,
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

    let mut response = match ServeFile::new(file_path)
        .oneshot(Request::new(Body::empty()))
        .await
    {
        Ok(response) => response.into_response(),
        Err(error) => match error {},
    };
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    Ok(response)
}

fn is_safe_hls_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
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
