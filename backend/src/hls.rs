use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, HeaderValue, Request},
    response::{IntoResponse, Response},
};
use tokio::process::Command;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{
    db::get_track,
    error::AppError,
    models::Track,
    state::AppState,
    util::{now_nanos, run_command_output},
};

const HLS_SEGMENT_SECONDS: &str = "6";

pub fn track_hls_cache_key(track: &Track) -> String {
    track.content_hash.clone()
}

pub fn track_hls_url(track: &Track) -> String {
    format!(
        "/api/v1/tracks/{}/hls/{}/index.m3u8",
        track.id,
        track_hls_cache_key(track)
    )
}

pub async fn get_hls_playlist(
    State(state): State<AppState>,
    AxumPath((id, cache_key)): AxumPath<(String, String)>,
) -> Result<Response, AppError> {
    if !is_safe_hls_token(&cache_key) {
        return Err(AppError::not_found("HLS playlist not found"));
    }
    let track = get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    if !PathBuf::from(&track.file_path).is_file() {
        return Err(AppError::not_found("Audio file not found"));
    }
    if cache_key != track_hls_cache_key(&track) {
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
    let track = get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    if !PathBuf::from(&track.file_path).is_file() {
        return Err(AppError::not_found("Audio file not found"));
    }
    if cache_key != track_hls_cache_key(&track) {
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

pub fn delete_hls_cache(state: &AppState, id: &str) {
    let path = state.data_dir.join(".cache").join("hls").join(id);
    if let Err(error) = fs::remove_dir_all(&path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(track_id = id, error = %error, "failed to delete HLS cache");
        }
    }
}

pub async fn prewarm_track_hls(state: &AppState, track: &Track) {
    if !PathBuf::from(&track.file_path).is_file() {
        return;
    }
    if let Err(error) = ensure_hls_cache(state, track, &track_hls_cache_key(track)).await {
        let filename = Path::new(&track.file_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&track.file_path);
        tracing::warn!(
            filename = filename,
            error = error.message,
            "failed to prewarm HLS cache"
        );
    }
}

async fn ensure_hls_cache(
    state: &AppState,
    track: &Track,
    cache_key: &str,
) -> Result<PathBuf, AppError> {
    let dir = hls_cache_dir(state, &track.id, cache_key);
    if hls_complete(&dir) {
        prune_stale_hls_cache(state, &track.id, cache_key);
        return Ok(dir);
    }

    let _guard = state
        .mutation_lock(format!("hls:{}:{cache_key}", track.id))
        .await;
    if hls_complete(&dir) {
        prune_stale_hls_cache(state, &track.id, cache_key);
        return Ok(dir);
    }

    prune_stale_hls_cache(state, &track.id, cache_key);
    generate_hls(track, &dir).await?;
    prune_stale_hls_cache(state, &track.id, cache_key);
    Ok(dir)
}

async fn generate_hls(track: &Track, dir: &Path) -> Result<()> {
    let parent = dir
        .parent()
        .ok_or_else(|| anyhow!("invalid HLS cache path"))?;
    fs::create_dir_all(parent)?;
    if dir.is_dir() {
        fs::remove_dir_all(dir)?;
    }
    let tmp = parent.join(format!(".hls-{}-{}", track.id, now_nanos()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;
    let source = fs::canonicalize(&track.file_path)?;

    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(source)
        .args([
            "-vn",
            "-map",
            "0:a:0",
            "-c:a",
            "aac",
            "-b:a",
            "160k",
            "-ac",
            "2",
            "-f",
            "hls",
            "-hls_time",
            HLS_SEGMENT_SECONDS,
            "-hls_playlist_type",
            "vod",
            "-hls_segment_type",
            "fmp4",
            "-hls_fmp4_init_filename",
            "init.mp4",
            "-hls_segment_filename",
            "segment-%05d.m4s",
        ])
        .arg("index.m3u8")
        .current_dir(&tmp)
        .stdout(Stdio::null());

    let output = run_command_output(command, Duration::from_secs(60 * 60 * 6)).await;
    match output {
        Ok(output) if output.status.success() && hls_complete(&tmp) => {
            fs::rename(&tmp, dir)?;
            Ok(())
        }
        Ok(output) if output.status.success() => {
            let _ = fs::remove_dir_all(&tmp);
            Err(anyhow!("ffmpeg created incomplete HLS audio output"))
        }
        Ok(output) => {
            let _ = fs::remove_dir_all(&tmp);
            Err(anyhow!(ffmpeg_error_message(&output)))
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&tmp);
            Err(error)
        }
    }
}

fn ffmpeg_error_message(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("ffmpeg failed to create HLS audio ({})", output.status)
    } else {
        format!(
            "ffmpeg failed to create HLS audio ({}): {}",
            output.status, stderr
        )
    }
}

fn hls_complete(dir: &Path) -> bool {
    dir.join("index.m3u8").is_file()
        && dir.join("init.mp4").is_file()
        && fs::read_to_string(dir.join("index.m3u8"))
            .is_ok_and(|playlist| playlist.contains("#EXT-X-ENDLIST"))
        && fs::read_dir(dir).is_ok_and(|entries| {
            entries.filter_map(Result::ok).any(|entry| {
                entry
                    .path()
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| is_safe_hls_media_file(value) && value.ends_with(".m4s"))
            })
        })
}

async fn serve_hls_file(
    file_path: PathBuf,
    content_type: &'static str,
    cache_control: &'static str,
    not_found: &'static str,
) -> Result<Response, AppError> {
    if !file_path.is_file() {
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

fn hls_cache_dir(state: &AppState, id: &str, cache_key: &str) -> PathBuf {
    state
        .data_dir
        .join(".cache")
        .join("hls")
        .join(id)
        .join(cache_key)
}

fn prune_stale_hls_cache(state: &AppState, id: &str, current_cache_key: &str) {
    let root = state.data_dir.join(".cache").join("hls").join(id);
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value != current_cache_key)
        {
            let _ = fs::remove_dir_all(path);
        }
    }
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
