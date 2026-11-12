//! HLS generation and cache lifecycle, independent of HTTP responses.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, Result};
use tokio::{fs, process::Command};

use crate::{models::Track, process::run_command_output, state::AppState};

use super::is_safe_hls_media_file;

const HLS_SEGMENT_SECONDS: &str = "6";

pub(crate) async fn prewarm_track_hls(state: &AppState, track: &Track) {
    if !fs::metadata(&track.file_path)
        .await
        .is_ok_and(|meta| meta.is_file())
    {
        return;
    }
    if let Err(error) = ensure_hls_cache(state, track, &track.content_hash).await {
        let filename = Path::new(&track.file_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&track.file_path);
        tracing::warn!(filename, error = %error, "failed to prewarm HLS cache");
    }
}

pub(super) async fn ensure_hls_cache(
    state: &AppState,
    track: &Track,
    cache_key: &str,
) -> Result<PathBuf> {
    let dir = hls_cache_dir(state, &track.id, cache_key);
    if hls_complete(&dir).await {
        prune_stale_hls_cache(state, &track.id, cache_key).await;
        return Ok(dir);
    }

    let _guard = state
        .mutation_lock(format!("hls:{}:{cache_key}", track.id))
        .await;
    if hls_complete(&dir).await {
        prune_stale_hls_cache(state, &track.id, cache_key).await;
        return Ok(dir);
    }

    prune_stale_hls_cache(state, &track.id, cache_key).await;
    generate_hls(track, &dir).await?;
    prune_stale_hls_cache(state, &track.id, cache_key).await;
    Ok(dir)
}

async fn generate_hls(track: &Track, dir: &Path) -> Result<()> {
    let parent = dir
        .parent()
        .ok_or_else(|| anyhow!("invalid HLS cache path"))?;
    fs::create_dir_all(parent).await?;
    if fs::metadata(dir).await.is_ok_and(|meta| meta.is_dir()) {
        fs::remove_dir_all(dir).await?;
    }
    let parent = parent.to_path_buf();
    let prefix = format!(".hls-{}-", track.id);
    let tmp = tokio::task::spawn_blocking(move || {
        tempfile::Builder::new().prefix(&prefix).tempdir_in(parent)
    })
    .await??;
    let source = fs::canonicalize(&track.file_path).await?;

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
        .current_dir(tmp.path())
        .stdout(Stdio::null());

    let result = async {
        let output = run_command_output(command, Duration::from_secs(60 * 60 * 6)).await?;
        if !output.status.success() {
            return Err(anyhow!(ffmpeg_error_message(&output)));
        }
        if !hls_complete(tmp.path()).await {
            return Err(anyhow!("ffmpeg created incomplete HLS audio output"));
        }
        fs::rename(tmp.path(), dir).await?;
        Ok(())
    }
    .await;
    let _ = tokio::task::spawn_blocking(move || tmp.close()).await;
    result
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

async fn hls_complete(dir: &Path) -> bool {
    let Ok(playlist) = fs::read_to_string(dir.join("index.m3u8")).await else {
        return false;
    };
    if !playlist.lines().any(|line| line.trim() == "#EXT-X-ENDLIST")
        || !nonempty_file(&dir.join("init.mp4")).await
    {
        return false;
    }
    let mut has_segment = false;
    for segment in playlist
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        if !is_safe_hls_media_file(segment)
            || !segment.ends_with(".m4s")
            || !nonempty_file(&dir.join(segment)).await
        {
            return false;
        }
        has_segment = true;
    }
    has_segment
}

async fn nonempty_file(path: &Path) -> bool {
    fs::metadata(path)
        .await
        .is_ok_and(|meta| meta.is_file() && meta.len() > 0)
}

pub(super) fn hls_cache_dir(state: &AppState, id: &str, cache_key: &str) -> PathBuf {
    state
        .data_dir
        .join(".cache")
        .join("hls")
        .join(id)
        .join(cache_key)
}

async fn prune_stale_hls_cache(state: &AppState, id: &str, current_cache_key: &str) {
    let root = state.data_dir.join(".cache").join("hls").join(id);
    let Ok(mut entries) = fs::read_dir(root).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value != current_cache_key)
        {
            let _ = fs::remove_dir_all(path).await;
        }
    }
}
