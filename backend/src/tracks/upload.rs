use std::{env, path::Path};

use axum::{
    extract::{Multipart, State},
    Json,
};
use serde_json::{json, Value};
use tempfile::TempPath;
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    artwork::download_artwork_as_jpeg,
    audio_names::{generate_filename, normalize_artists, normalize_text},
    error::AppError,
    files::can_write,
    media::{apply_audio_update, AudioUpdate},
    scanner::upsert_path,
    state::AppState,
};

#[derive(Default)]
struct Upload {
    title: Option<String>,
    artist: Option<String>,
    artwork_url: Option<String>,
    file: Option<TempPath>,
}

impl Upload {
    async fn read(mut multipart: Multipart, cache_dir: &Path) -> Result<Self, AppError> {
        let mut upload = Self::default();
        while let Some(mut field) = multipart.next_field().await? {
            match field.name().unwrap_or_default() {
                "title" => upload.title = Some(field.text().await?),
                "artist" => upload.artist = Some(field.text().await?),
                "artwork_url" => upload.artwork_url = Some(field.text().await?),
                "file" => {
                    let ext = Path::new(field.file_name().unwrap_or_default())
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if !matches!(ext.as_str(), "mp3" | "flac" | "wav") {
                        return Err(AppError::bad_request(
                            "Unsupported file format. Only MP3, FLAC, and WAV are supported.",
                        ));
                    }
                    let max_size_mb = env::var("MAX_UPLOAD_SIZE_MB")
                        .ok()
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(30);
                    let max_bytes = max_size_mb
                        .checked_mul(1024 * 1024)
                        .ok_or_else(|| AppError::bad_request("MAX_UPLOAD_SIZE_MB is too large"))?;
                    fs::create_dir_all(cache_dir).await?;
                    let directory = cache_dir.to_path_buf();
                    let (file, path) = tokio::task::spawn_blocking(move || {
                        tempfile::Builder::new()
                            .prefix("upload-")
                            .suffix(&format!(".{ext}"))
                            .tempfile_in(directory)
                            .map(|file| file.into_parts())
                    })
                    .await
                    .map_err(anyhow::Error::from)??;
                    let mut file = fs::File::from_std(file);
                    let mut remaining = max_bytes;
                    while let Some(chunk) = field.chunk().await? {
                        remaining = remaining.checked_sub(chunk.len()).ok_or_else(|| {
                            AppError::payload_too_large(format!(
                                "File size exceeds maximum allowed size ({max_size_mb}MB)"
                            ))
                        })?;
                        file.write_all(&chunk).await?;
                    }
                    file.flush().await?;
                    // Close the file before its path guard, including on Windows.
                    drop(file);
                    upload.file = Some(path);
                }
                _ => {}
            }
        }
        Ok(upload)
    }
}

pub async fn upload_track(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    if !can_write(&state.music_dir).await {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    let cache_dir = state.data_dir.join(".cache");
    let upload = Upload::read(multipart, &cache_dir).await?;
    let title = normalize_text(
        &upload
            .title
            .ok_or_else(|| AppError::bad_request("Missing title"))?,
    );
    let artist = normalize_artists(
        &upload
            .artist
            .ok_or_else(|| AppError::bad_request("Missing artist"))?,
    );
    let file = upload
        .file
        .ok_or_else(|| AppError::bad_request("Missing file"))?;
    let ext = file.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    let path = state
        .music_dir
        .join(generate_filename(&artist, &title, ext)?);
    if fs::try_exists(&path).await? {
        return Err(AppError::conflict("A file with this name already exists"));
    }
    let jpeg_path = match upload.artwork_url.filter(|url| !url.trim().is_empty()) {
        Some(url) => Some(download_artwork_as_jpeg(&url, &cache_dir).await?),
        None => None,
    };
    apply_audio_update(
        &file,
        &cache_dir,
        AudioUpdate {
            title: Some(&title),
            artist: Some(&artist),
            cover_jpeg_path: jpeg_path.as_deref(),
            target_path: Some(&path),
            standardize_tags: false,
            update_mtime: true,
            remove_section: None,
        },
    )
    .await?;
    drop(jpeg_path);
    let (track, _) = upsert_path(&state, &path, Some(title), Some(artist)).await?;
    tracing::info!(track_id = track.id, "track uploaded");
    Ok(Json(
        json!({"success": true, "message": "File uploaded and queued for processing."}),
    ))
}
