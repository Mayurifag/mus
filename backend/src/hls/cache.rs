//! HLS generation and cache lifecycle, independent of HTTP responses.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use tokio::{fs, process::Command};

use crate::{process::run_command_output, state::AppState};

use super::is_hls_segment;

const HLS_SEGMENT_SECONDS: &str = "6";

pub(super) async fn ensure_hls_cache(state: &AppState, id: &str, source: &Path) -> Result<PathBuf> {
    let dir = hls_cache_dir(state, id);
    if hls_complete(&dir).await? {
        return Ok(dir);
    }

    let _guard = state.mutation_lock(format!("hls:{id}")).await;
    if hls_complete(&dir).await? {
        return Ok(dir);
    }

    generate_hls(id, source, &dir).await?;
    Ok(dir)
}

async fn generate_hls(id: &str, source: &Path, dir: &Path) -> Result<()> {
    let parent = dir
        .parent()
        .ok_or_else(|| anyhow!("invalid HLS cache path"))?;
    fs::create_dir_all(parent).await?;
    if fs::metadata(dir).await.is_ok_and(|meta| meta.is_dir()) {
        fs::remove_dir_all(dir).await?;
    }
    let parent = parent.to_path_buf();
    let prefix = format!(".hls-{id}-");
    let tmp = tokio::task::spawn_blocking(move || {
        tempfile::Builder::new().prefix(&prefix).tempdir_in(parent)
    })
    .await??;
    let source = fs::canonicalize(source).await?;

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
        if !hls_complete(tmp.path()).await? {
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

async fn hls_complete(dir: &Path) -> Result<bool> {
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || validate_hls_cache(&dir))
        .await
        .context("HLS cache validation worker failed")
}

fn validate_hls_cache(dir: &Path) -> bool {
    let Ok(playlist) = std::fs::read_to_string(dir.join("index.m3u8")) else {
        return false;
    };
    let mut lines = playlist.trim_end().lines().map(str::trim);
    if lines.next() != Some("#EXTM3U")
        || lines.next_back() != Some("#EXT-X-ENDLIST")
        || !nonempty_file(&dir.join("init.mp4"))
    {
        return false;
    }
    let mut has_init = false;
    let mut has_segment = false;
    for line in lines {
        if line.starts_with("#EXT-X-MAP:") {
            if line != "#EXT-X-MAP:URI=\"init.mp4\"" {
                return false;
            }
            has_init = true;
        } else if !line.is_empty() && !line.starts_with('#') {
            if !has_init || !is_hls_segment(line) || !nonempty_file(&dir.join(line)) {
                return false;
            }
            has_segment = true;
        }
    }
    has_segment
}

fn nonempty_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() > 0)
}

pub(super) fn hls_cache_dir(state: &AppState, id: &str) -> PathBuf {
    state.data_dir.join(".cache/hls").join(id)
}
