//! Media download, staged artwork, publication, and library registration.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use tokio::{fs, process::Command};

use super::{diagnostics::yt_dlp_output, publish};
use crate::{
    artwork::download_artwork_as_jpeg,
    audio_names::{generate_filename, normalize_artists, normalize_text},
    db,
    files::audio_paths,
    scanner::upsert_path,
    state::AppState,
};

#[derive(Default)]
pub(super) enum DownloadNaming {
    #[default]
    Source,
    Custom {
        title: String,
        artist: String,
    },
}

#[derive(Default)]
pub(super) struct DownloadOptions {
    pub naming: DownloadNaming,
    pub artwork_url: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub(super) async fn run_download(
    state: &AppState,
    url: String,
    options: DownloadOptions,
) -> Result<PathBuf> {
    let cache_dir = state.data_dir.join(".cache");
    fs::create_dir_all(&cache_dir).await?;
    let tmp = tokio::task::spawn_blocking(move || {
        tempfile::Builder::new()
            .prefix("download-")
            .tempdir_in(cache_dir)
    })
    .await??;
    let command = download_command(&url, tmp.path(), options.artwork_url.is_none());
    let result = run_download_inner(state, command, options, tmp.path()).await;
    let _ = tokio::task::spawn_blocking(move || tmp.close()).await;
    result
}

pub(super) async fn run_download_inner(
    state: &AppState,
    command: Command,
    options: DownloadOptions,
    tmp: &Path,
) -> Result<PathBuf> {
    let DownloadOptions {
        naming,
        artwork_url,
        tags,
    } = options;
    let (title, artist) = match naming {
        DownloadNaming::Source => (None, None),
        DownloadNaming::Custom { title, artist } => (
            Some(normalize_text(&title)),
            Some(normalize_artists(&artist)),
        ),
    };
    let media = async {
        let started = Instant::now();
        let result = yt_dlp_output(command, Duration::from_secs(600)).await;
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis(),
            success = result.is_ok(),
            "download media processing finished"
        );
        result
    };
    let artwork = async {
        let started = Instant::now();
        let result = match artwork_url {
            Some(url) => download_artwork_as_jpeg(&url, tmp).await.map(Some),
            None => Ok(None),
        };
        tracing::info!(
            elapsed_ms = started.elapsed().as_millis(),
            success = result.is_ok(),
            "download artwork processing finished"
        );
        result
    };
    let (media_result, artwork_result) = tokio::join!(media, artwork);
    media_result?;
    let jpeg_path = artwork_result?;
    let directory = tmp.to_path_buf();
    let downloaded = tokio::task::spawn_blocking(move || audio_paths(&directory))
        .await??
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Downloaded file not found"))?;
    let final_name = match (&title, &artist) {
        (Some(title), Some(artist)) => generate_filename(artist, title, "mp3")?,
        _ => downloaded
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("download.mp3")
            .to_string(),
    };
    let mut final_path = state.music_dir.join(final_name);
    let started = Instant::now();
    publish::publish_download(
        &downloaded,
        tmp,
        &final_path,
        title.as_deref(),
        artist.as_deref(),
        jpeg_path.as_deref(),
    )
    .await?;
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis(),
        "download tags and publication finished"
    );
    let published_metadata = fs::metadata(&final_path).await?;
    let started = Instant::now();
    let (track, accepted_path) = upsert_path(state, &final_path, title, artist).await?;
    let registered = db::get_track(state, &track.id)
        .await?
        .ok_or_else(|| anyhow!("downloaded track disappeared during registration"))?;
    let registered_path = PathBuf::from(&registered.file_path);
    if accepted_path.as_ref() == Some(&registered_path)
        && same_file(&published_metadata, &fs::metadata(&registered_path).await?)
    {
        if let Some(tags) = tags {
            anyhow::ensure!(
                db::set_download_tags(state, &track.id, &registered_path, &tags).await?,
                "downloaded track moved before categories could be saved"
            );
        }
        final_path = registered_path;
    } else {
        tracing::info!(
            track_id = track.id,
            "download shares an existing track; its tags were not changed"
        );
    }
    tracing::info!(
        track_id = track.id,
        elapsed_ms = started.elapsed().as_millis(),
        "download library registration and HLS prewarm finished"
    );
    Ok(final_path)
}

#[cfg(unix)]
fn same_file(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(_: &std::fs::Metadata, _: &std::fs::Metadata) -> bool {
    // The scanner confirms this path belongs to the published file rather than a duplicate.
    true
}

pub(super) fn download_command(url: &str, tmp: &Path, embed_thumbnail: bool) -> Command {
    let output_template = tmp.join("%(artist,uploader|Unknown Artist)s - %(title)s.%(ext)s");
    let mut cmd = Command::new("yt-dlp");
    cmd.args([
        "--format",
        "bestaudio/best",
        "--extract-audio",
        "--audio-format",
        "mp3",
        "--audio-quality",
        "0",
        "-o",
    ])
    .arg(output_template);
    if embed_thumbnail {
        cmd.args(["--embed-thumbnail", "--convert-thumbnails", "jpg"]);
    }
    cmd.args([
        "--embed-metadata",
        "--parse-metadata",
        "title:%(title)s",
        "--parse-metadata",
        "artist:%(artist,uploader|Unknown Artist)s",
        "--sponsorblock-remove",
        "all",
        "--embed-chapters",
        "--concurrent-fragments",
        "3",
        "--throttled-rate",
        "100K",
        "--retries",
        "10",
        "--no-playlist",
        "--",
        url,
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    cmd
}
