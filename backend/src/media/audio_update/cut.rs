//! Format-specific cut encoding and preservation of embedded metadata.

use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use id3::{TagLike, Version};
use tempfile::TempPath;
use tokio::process::Command;

use super::{staging_file, AudioUpdate};
use crate::{
    media::{
        cut::{cut_filter, read_cut_source, validate_cut},
        id3_tags::{read_for_update, supports_id3, update_tag},
    },
    models::AudioCut,
    process::run_command_status,
};

pub(super) async fn cut_audio_to_temp(
    path: &Path,
    cache_dir: &Path,
    update: &AudioUpdate<'_>,
    cut: AudioCut,
) -> Result<TempPath> {
    let source = read_cut_source(path).await?;
    validate_cut(cut, source.duration)?;
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut command = Command::new("ffmpeg");
    command
        .args(["-v", "error", "-nostdin", "-y", "-i"])
        .arg(path);
    let id3 = supports_id3(path);
    if !id3 && let Some(cover) = update.cover_jpeg_path {
        command.arg("-i").arg(cover);
    }
    command.args([
        "-filter_complex",
        &cut_filter(cut, source.duration),
        "-map",
        "[out]",
    ]);
    if ext != "wav" {
        command.args([
            "-map",
            if !id3 && update.cover_jpeg_path.is_some() {
                "1:v:0"
            } else {
                "0:v?"
            },
            "-c:v",
            "copy",
            "-disposition:v",
            "attached_pic",
        ]);
    }
    command.args([
        "-map_metadata",
        "0",
        "-map_metadata:s:a:0",
        "0:s:a:0",
        "-map_chapters",
        "-1",
    ]);
    match ext.as_str() {
        "mp3" => {
            command.args(["-c:a", "libmp3lame", "-q:a", "2", "-id3v2_version", "3"]);
        }
        "flac" => {
            command.args(["-c:a", "flac"]);
        }
        "wav" => {
            command.args([
                "-c:a",
                if source.codec.starts_with("pcm_") {
                    &source.codec
                } else {
                    "pcm_s24le"
                },
            ]);
        }
        "m4a" => {
            if source.codec == "alac" {
                command.args(["-c:a", "alac"]);
            } else {
                command.args(["-c:a", "aac", "-b:a", "256k"]);
            }
        }
        "ogg" => {
            command.args(["-c:a", "libvorbis", "-q:a", "6"]);
        }
        _ => return Err(anyhow!("Cutting this audio format is not supported")),
    }
    if let Some(title) = update.title {
        command.arg("-metadata").arg(format!("title={title}"));
        command.arg("-metadata:s:a:0").arg(format!("title={title}"));
    }
    if let Some(artist) = update.artist {
        command.arg("-metadata").arg(format!("artist={artist}"));
        command
            .arg("-metadata:s:a:0")
            .arg(format!("artist={artist}"));
    }
    let tmp = staging_file(cache_dir, "cut", path).await?;
    command
        .arg(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = run_command_status(command, Duration::from_secs(300)).await?;
    if !status.success() || tokio::fs::metadata(&tmp).await?.len() == 0 {
        return Err(anyhow!("ffmpeg failed to cut audio"));
    }
    let tmp = if id3 {
        let path = path.to_path_buf();
        let title = update.title.map(str::to_owned);
        let artist = update.artist.map(str::to_owned);
        let cover = update.cover_jpeg_path.map(Path::to_path_buf);
        tokio::task::spawn_blocking(move || -> Result<TempPath> {
            let mut tag = read_for_update(&path)?;
            tag.remove("CHAP");
            tag.remove("CTOC");
            tag.remove("TLEN");
            update_tag(
                &mut tag,
                title.as_deref(),
                artist.as_deref(),
                cover.as_deref(),
            )?;
            tag.write_to_path(&tmp, Version::Id3v23)?;
            Ok(tmp)
        })
        .await??
    } else {
        tmp
    };
    read_cut_source(&tmp).await?;
    Ok(tmp)
}
