use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, Result};
use id3::{frame::Picture, frame::PictureType, Tag, TagLike, Version};
use serde_json::Value;
use tokio::process::Command;

use crate::util::{now_nanos, run_command_output, run_command_status};

#[derive(Debug)]
pub struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub duration: i64,
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

pub async fn write_audio_tags(
    path: &Path,
    cache_dir: &Path,
    title: &str,
    artist: &str,
) -> Result<()> {
    if supports_id3(path) {
        let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());
        tag.set_title(title);
        tag.set_artist(artist);
        tag.write_to_path(path, Version::Id3v23)?;
        return Ok(());
    }
    rewrite_audio(
        path,
        cache_dir,
        vec![
            "-metadata".to_string(),
            format!("title={title}"),
            "-metadata".to_string(),
            format!("artist={artist}"),
        ],
    )
    .await
}

pub async fn write_audio_cover(path: &Path, cache_dir: &Path, jpeg_path: &Path) -> Result<()> {
    if supports_id3(path) {
        let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());
        tag.remove_all_pictures();
        tag.add_frame(Picture {
            mime_type: "image/jpeg".into(),
            picture_type: PictureType::CoverFront,
            description: String::new(),
            data: fs::read(jpeg_path)?,
        });
        tag.write_to_path(path, Version::Id3v23)?;
        drop(tag);
        return Ok(());
    }

    rewrite_audio_cover(path, cache_dir, jpeg_path).await
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

async fn rewrite_audio(path: &Path, cache_dir: &Path, extra_args: Vec<String>) -> Result<()> {
    let tmp = allocate_path(cache_dir, "tags", path)?;
    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-i"])
        .arg(path)
        .args(["-map", "0", "-map_metadata", "0", "-c", "copy"]);
    command
        .args(&extra_args)
        .arg(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = match run_command_status(command, Duration::from_secs(300)).await {
        Ok(status) => status,
        Err(error) => {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
    };
    if status.success() && tmp.is_file() {
        if let Err(error) = replace_file(&tmp, path) {
            let _ = fs::remove_file(tmp);
            return Err(error);
        }
    } else {
        let _ = fs::remove_file(tmp);
        return Err(anyhow!("ffmpeg failed to rewrite audio tags"));
    }
    Ok(())
}

async fn rewrite_audio_cover(path: &Path, cache_dir: &Path, jpeg_path: &Path) -> Result<()> {
    let tmp = allocate_path(cache_dir, "cover", path)?;
    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-i"])
        .arg(path)
        .args(["-i"])
        .arg(jpeg_path)
        .args([
            "-map",
            "0",
            "-map",
            "1",
            "-map_metadata",
            "0",
            "-c",
            "copy",
            "-c:v",
            "mjpeg",
            "-disposition:v",
            "attached_pic",
        ])
        .arg(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = match run_command_status(command, Duration::from_secs(300)).await {
        Ok(status) => status,
        Err(error) => {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
    };
    if status.success() && tmp.is_file() {
        if let Err(error) = replace_file(&tmp, path) {
            let _ = fs::remove_file(tmp);
            return Err(error);
        }
    } else {
        let _ = fs::remove_file(tmp);
        return Err(anyhow!("ffmpeg failed to embed cover art"));
    }
    Ok(())
}

fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            copy_over(source, destination)?;
            let _ = fs::remove_file(source);
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn copy_over(source: &Path, destination: &Path) -> Result<()> {
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(destination)?;
    io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    Ok(())
}

fn allocate_path(dir: &Path, kind: &str, path: &Path) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    for attempt in 0..100 {
        let mut file_name = format!(".mus-{kind}-{}-{attempt}", now_nanos());
        if let Some(ext) = path
            .extension()
            .and_then(|v| v.to_str())
            .filter(|v| !v.is_empty())
        {
            file_name.push('.');
            file_name.push_str(ext);
        }
        let path = dir.join(file_name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(anyhow!("failed to allocate rewrite file"))
}
