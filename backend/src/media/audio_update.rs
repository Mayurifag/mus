use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, Result};
use filetime::{set_file_mtime, FileTime};
use id3::{frame::Picture, frame::PictureType, Tag, TagLike, Version};
use tokio::process::Command;

use crate::util::{now_nanos, run_command_status};

pub struct AudioUpdate<'a> {
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub cover_jpeg_path: Option<&'a Path>,
    pub target_path: Option<&'a Path>,
    pub standardize_tags: bool,
    pub update_mtime: bool,
}

pub struct AudioUpdateResult {
    pub path: PathBuf,
    pub changed: bool,
}

pub async fn apply_audio_update(
    path: &Path,
    cache_dir: &Path,
    update: AudioUpdate<'_>,
) -> Result<AudioUpdateResult> {
    let destination = update.target_path.unwrap_or(path);
    let path_changed = destination != path;
    let standardize_id3 = update.standardize_tags && needs_id3_standardization(path);
    let media_changed = update.title.is_some()
        || update.artist.is_some()
        || update.cover_jpeg_path.is_some()
        || standardize_id3;

    if !path_changed && !media_changed {
        return Ok(AudioUpdateResult {
            path: path.to_path_buf(),
            changed: false,
        });
    }

    let modified = update.update_mtime.then(FileTime::now);
    if media_changed {
        let tmp = if supports_id3(path) {
            rewrite_id3_to_temp(path, cache_dir, &update, standardize_id3)?
        } else {
            rewrite_audio_to_temp(path, cache_dir, &update).await?
        };
        if let Some(modified) = modified {
            set_file_mtime(&tmp, modified)?;
        }
        if let Err(error) = publish_updated_file(&tmp, path, destination, modified) {
            let _ = fs::remove_file(tmp);
            return Err(error);
        }
    } else {
        move_path_and_touch(path, destination, modified)?;
    }

    Ok(AudioUpdateResult {
        path: destination.to_path_buf(),
        changed: true,
    })
}

pub async fn standardize_audio_tags(path: &Path, cache_dir: &Path) -> Result<()> {
    apply_audio_update(
        path,
        cache_dir,
        AudioUpdate {
            title: None,
            artist: None,
            cover_jpeg_path: None,
            target_path: None,
            standardize_tags: true,
            update_mtime: true,
        },
    )
    .await?;
    Ok(())
}

pub async fn write_audio_tags(
    path: &Path,
    cache_dir: &Path,
    title: &str,
    artist: &str,
) -> Result<()> {
    apply_audio_update(
        path,
        cache_dir,
        AudioUpdate {
            title: Some(title),
            artist: Some(artist),
            cover_jpeg_path: None,
            target_path: None,
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await?;
    Ok(())
}

pub async fn write_audio_cover(path: &Path, cache_dir: &Path, jpeg_path: &Path) -> Result<()> {
    apply_audio_update(
        path,
        cache_dir,
        AudioUpdate {
            title: None,
            artist: None,
            cover_jpeg_path: Some(jpeg_path),
            target_path: None,
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await?;
    Ok(())
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

fn needs_id3_standardization(path: &Path) -> bool {
    supports_id3(path)
        && Tag::read_from_path(path)
            .unwrap_or_else(|_| Tag::new())
            .version()
            != Version::Id3v23
}

fn rewrite_id3_to_temp(
    path: &Path,
    cache_dir: &Path,
    update: &AudioUpdate<'_>,
    standardize_id3: bool,
) -> Result<PathBuf> {
    let tmp = allocate_path(cache_dir, "audio", path)?;
    if let Err(error) = copy_over(path, &tmp).and_then(|_| {
        let mut tag = Tag::read_from_path(&tmp).unwrap_or_else(|_| Tag::new());
        if let Some(title) = update.title {
            tag.set_title(title);
        }
        if let Some(artist) = update.artist {
            tag.set_artist(artist);
        }
        if let Some(jpeg_path) = update.cover_jpeg_path {
            tag.remove_all_pictures();
            tag.add_frame(Picture {
                mime_type: "image/jpeg".into(),
                picture_type: PictureType::CoverFront,
                description: String::new(),
                data: fs::read(jpeg_path)?,
            });
        }
        if standardize_id3
            || update.title.is_some()
            || update.artist.is_some()
            || update.cover_jpeg_path.is_some()
        {
            tag.write_to_path(&tmp, Version::Id3v23)?;
        }
        Ok(())
    }) {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }
    Ok(tmp)
}

async fn rewrite_audio_to_temp(
    path: &Path,
    cache_dir: &Path,
    update: &AudioUpdate<'_>,
) -> Result<PathBuf> {
    let tmp = allocate_path(cache_dir, "audio", path)?;
    let mut command = Command::new("ffmpeg");
    command.args(["-y", "-i"]).arg(path);
    if let Some(jpeg_path) = update.cover_jpeg_path {
        command.args(["-i"]).arg(jpeg_path);
    }
    command.args(["-map", "0", "-map_metadata", "0", "-c", "copy"]);
    if update.cover_jpeg_path.is_some() {
        command.args([
            "-map",
            "1",
            "-c:v",
            "mjpeg",
            "-disposition:v",
            "attached_pic",
        ]);
    }
    if let Some(title) = update.title {
        command.arg("-metadata").arg(format!("title={title}"));
    }
    if let Some(artist) = update.artist {
        command.arg("-metadata").arg(format!("artist={artist}"));
    }
    command
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
        Ok(tmp)
    } else {
        let _ = fs::remove_file(tmp);
        Err(anyhow!("ffmpeg failed to rewrite audio"))
    }
}

fn publish_updated_file(
    source: &Path,
    original: &Path,
    destination: &Path,
    modified: Option<FileTime>,
) -> Result<()> {
    if destination != original && destination.exists() {
        return Err(anyhow!("A file with this name already exists"));
    }
    replace_file(source, original, modified)?;
    if destination != original {
        move_path_and_touch(original, destination, None)?;
    }
    Ok(())
}

fn replace_file(source: &Path, destination: &Path, modified: Option<FileTime>) -> Result<()> {
    copy_over(source, destination)?;
    let _ = fs::remove_file(source);
    if let Some(modified) = modified {
        set_file_mtime(destination, modified)?;
    }
    Ok(())
}

fn move_path_and_touch(
    source: &Path,
    destination: &Path,
    modified: Option<FileTime>,
) -> Result<()> {
    if source == destination {
        if let Some(modified) = modified {
            set_file_mtime(destination, modified)?;
        }
        return Ok(());
    }
    if destination.exists() {
        return Err(anyhow!("A file with this name already exists"));
    }
    match fs::rename(source, destination) {
        Ok(()) => {
            if let Some(modified) = modified {
                set_file_mtime(destination, modified)?;
            }
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            copy_without_replace(source, destination)?;
            fs::remove_file(source)?;
            if let Some(modified) = modified {
                set_file_mtime(destination, modified)?;
            }
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

fn copy_without_replace(source: &Path, destination: &Path) -> Result<()> {
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    if let Err(error) = io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
        let _ = fs::remove_file(destination);
        return Err(error.into());
    }
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
