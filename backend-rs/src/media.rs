use std::{fs, path::Path, process::Stdio};

use anyhow::{anyhow, Result};
use id3::{Tag, TagLike, Version};
use serde_json::Value;
use tokio::process::Command;

#[derive(Debug)]
pub struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub duration: i64,
}

pub async fn read_metadata(path: &Path) -> Result<MediaMetadata> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            path.to_str().unwrap_or_default(),
        ])
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow!("ffprobe failed"));
    }
    let data: Value = serde_json::from_slice(&output.stdout)?;
    let format = data.get("format").unwrap_or(&Value::Null);
    let tags = format.get("tags").unwrap_or(&Value::Null);
    Ok(MediaMetadata {
        title: tags
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or("Unknown Title")
            })
            .to_string(),
        artist: tags
            .get("artist")
            .and_then(Value::as_str)
            .unwrap_or("Unknown Artist")
            .to_string(),
        duration: format
            .get("duration")
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<f64>().ok())
            .map(|v| v as i64)
            .unwrap_or(0),
    })
}

pub async fn extract_cover(path: &Path, covers_dir: &Path, id: i64) -> Result<bool> {
    let original = covers_dir.join(format!("{id}_original.webp"));
    let small = covers_dir.join(format!("{id}_small.webp"));
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(path)
        .args(["-an", "-vframes", "1", "-vcodec", "libwebp", "-q:v", "85"])
        .arg(&original)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;
    if !status.success() || !original.is_file() {
        let _ = fs::remove_file(original);
        let _ = fs::remove_file(small);
        return Ok(false);
    }
    let small_status = Command::new("ffmpeg")
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
        .stderr(Stdio::null())
        .status()
        .await?;
    if !small_status.success() || !small.is_file() {
        let _ = fs::remove_file(original);
        let _ = fs::remove_file(small);
        return Ok(false);
    }
    Ok(true)
}

pub async fn standardize_audio_tags(path: &Path) -> Result<()> {
    if supports_id3(path) {
        let tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());
        if tag.version() == Version::Id3v23 {
            return Ok(());
        }
        tag.write_to_path(path, Version::Id3v23)?;
        return Ok(());
    }
    Ok(())
}

pub async fn write_audio_tags(path: &Path, title: &str, artist: &str) -> Result<()> {
    if supports_id3(path) {
        let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());
        tag.set_title(title);
        tag.set_artist(artist);
        tag.write_to_path(path, Version::Id3v23)?;
        return Ok(());
    }
    rewrite_audio(
        path,
        vec![
            "-metadata".to_string(),
            format!("title={title}"),
            "-metadata".to_string(),
            format!("artist={artist}"),
        ],
    )
    .await
}

fn supports_id3(path: &Path) -> bool {
    path.extension()
        .and_then(|v| v.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "aif" | "aiff"
            )
        })
}

async fn rewrite_audio(path: &Path, extra_args: Vec<String>) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let Some(file_name) = path.file_name().and_then(|v| v.to_str()) else {
        return Ok(());
    };
    let tmp = parent.join(format!(".mus-tags-{file_name}"));
    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-i"])
        .arg(path)
        .args(["-map", "0", "-map_metadata", "0", "-c", "copy"]);
    let status = command
        .args(&extra_args)
        .arg(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;
    if status.success() && tmp.is_file() {
        fs::rename(tmp, path)?;
    } else {
        let _ = fs::remove_file(tmp);
        return Err(anyhow!("ffmpeg failed to rewrite audio tags"));
    }
    Ok(())
}
