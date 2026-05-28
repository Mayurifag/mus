use std::{fs, path::Path, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::process::Command;

use crate::util::{normalize_artists, normalize_text, run_command_output, run_command_status};

mod audio_update;

pub use audio_update::{
    apply_audio_update, standardize_audio_tags, write_audio_cover, write_audio_tags, AudioUpdate,
    AudioUpdateResult,
};

#[derive(Debug)]
pub struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub duration: i64,
    pub has_artist: bool,
}

pub async fn read_metadata(path: &Path) -> Result<MediaMetadata> {
    let mut command = Command::new("ffprobe");
    command.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        path.to_str().unwrap_or_default(),
    ]);
    let output = run_command_output(command, Duration::from_secs(30)).await?;
    if !output.status.success() {
        return Err(anyhow!("ffprobe failed"));
    }
    let data: Value = serde_json::from_slice(&output.stdout)?;
    let format = data.get("format").unwrap_or(&Value::Null);
    let tags = format.get("tags").unwrap_or(&Value::Null);
    let title = tags.get("title").and_then(Value::as_str);
    let artist = tags.get("artist").and_then(Value::as_str);
    Ok(MediaMetadata {
        title: normalize_text(title.unwrap_or_else(|| {
            path.file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("Unknown Title")
        })),
        artist: normalize_artists(artist.unwrap_or("Unknown Artist")),
        duration: format
            .get("duration")
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .map(|v| v as i64)
            .unwrap_or(0),
        has_artist: artist.is_some(),
    })
}

pub async fn extract_cover(path: &Path, covers_dir: &Path, id: i64) -> Result<bool> {
    let original = covers_dir.join(format!("{id}_original.webp"));
    let small = covers_dir.join(format!("{id}_small.webp"));
    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-i"])
        .arg(path)
        .args(["-an", "-vframes", "1", "-vcodec", "libwebp", "-q:v", "85"])
        .arg(&original)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = run_command_status(command, Duration::from_secs(120)).await?;
    if !status.success() || !original.is_file() {
        let _ = fs::remove_file(original);
        let _ = fs::remove_file(small);
        return Ok(false);
    }
    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-i"])
        .arg(&original)
        .args([
            "-vf",
            "scale=80:80:force_original_aspect_ratio=increase,crop=80:80",
            "-vcodec",
            "libwebp",
            "-q:v",
            "80",
        ])
        .arg(&small)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let small_status = run_command_status(command, Duration::from_secs(120)).await?;
    if !small_status.success() || !small.is_file() {
        let _ = fs::remove_file(original);
        let _ = fs::remove_file(small);
        return Ok(false);
    }
    Ok(true)
}
