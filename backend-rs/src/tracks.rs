use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use axum::{
    body::Body,
    extract::{Multipart, Path as AxumPath, State},
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use tokio::{fs as async_fs, io as async_io, io::AsyncWriteExt};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{
    artwork::download_artwork_as_jpeg,
    db::{self, delete_track_row, get_track, save_track, track_dto},
    error::AppError,
    events::broadcast,
    media::{extract_cover, write_audio_cover, write_audio_tags},
    models::{TrackDto, TrackUpdate},
    scanner::upsert_path,
    state::AppState,
    util::{
        can_write, file_content_hash, file_signature, generate_filename, inode, now, now_nanos,
    },
};

pub async fn get_tracks(State(state): State<AppState>) -> Result<Json<Vec<TrackDto>>, AppError> {
    let tracks = db::list_tracks(&state)?;
    Ok(Json(tracks.into_iter().map(track_dto).collect()))
}

pub async fn stream_track(
    request_headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Response, AppError> {
    let track = get_track(&state, id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    if !Path::new(&track.file_path).is_file() {
        return Err(AppError::not_found("Audio file not found"));
    }
    let content_type = mime_guess::from_path(&track.file_path).first_or_octet_stream();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type.as_ref()).unwrap(),
    );
    let mut request = Request::new(Body::empty());
    *request.headers_mut() = request_headers;
    Ok((
        headers,
        ServeFile::new(track.file_path)
            .oneshot(request)
            .await
            .unwrap(),
    )
        .into_response())
}

pub async fn get_cover_small(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Response, AppError> {
    cover_response(state, id, "small").await
}

pub async fn get_cover_original(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> Result<Response, AppError> {
    cover_response(state, id, "original").await
}

async fn cover_response(state: AppState, id: i64, size: &str) -> Result<Response, AppError> {
    let cover_path = state.covers_dir.join(format!("{id}_{size}.webp"));
    if !cover_path.is_file() {
        return Err(AppError::not_found("Cover not found"));
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    Ok((
        headers,
        ServeFile::new(cover_path)
            .oneshot(Request::new(Body::empty()))
            .await
            .unwrap(),
    )
        .into_response())
}

pub async fn update_track(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
    Json(update): Json<TrackUpdate>,
) -> Result<Json<Value>, AppError> {
    if !can_write(&state.music_dir) {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    let track = get_track(&state, id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let new_title = update.title.unwrap_or_else(|| track.title.clone());
    let new_artist = update.artist.unwrap_or_else(|| track.artist.clone());
    let rename_file = update.rename_file.unwrap_or(false);
    let artwork_url = update.artwork_url.filter(|url| !url.trim().is_empty());

    if new_title == track.title
        && new_artist == track.artist
        && !rename_file
        && artwork_url.is_none()
    {
        return Ok(Json(json!({"status": "no_changes"})));
    }

    let mut new_path = PathBuf::from(&track.file_path);
    if rename_file {
        let ext = new_path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("mp3");
        let filename = generate_filename(&new_artist, &new_title, ext)?;
        new_path = new_path.parent().unwrap_or(&state.music_dir).join(filename);
        if new_path != Path::new(&track.file_path) && new_path.exists() {
            return Err(AppError::conflict("A file with this name already exists"));
        }
    }

    tokio::spawn(async move {
        match apply_track_update(
            state.clone(),
            id,
            new_title,
            new_artist,
            rename_file,
            artwork_url,
        )
        .await
        {
            Ok(dto) => {
                tracing::info!(track_id = dto.id, rename_file, "track updated");
                broadcast(
                    &state,
                    "track_updated",
                    Some("Track updated"),
                    Some("success"),
                    Some(json!(dto)),
                );
            }
            Err(error) => {
                let message = error.message;
                tracing::error!(
                    track_id = id,
                    error = message.as_str(),
                    "track update failed"
                );
                broadcast(
                    &state,
                    "track_update_failed",
                    Some("Failed to update track"),
                    Some("error"),
                    Some(json!({"id": id, "error": message})),
                );
            }
        }
    });

    Ok(Json(json!({"status": "queued"})))
}

async fn apply_track_update(
    state: AppState,
    id: i64,
    new_title: String,
    new_artist: String,
    rename_file: bool,
    artwork_url: Option<String>,
) -> Result<TrackDto, AppError> {
    let mut track = get_track(&state, id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let mut new_path = PathBuf::from(&track.file_path);
    if rename_file {
        let ext = new_path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("mp3");
        let filename = generate_filename(&new_artist, &new_title, ext)?;
        new_path = new_path.parent().unwrap_or(&state.music_dir).join(filename);
        if new_path != Path::new(&track.file_path) && new_path.exists() {
            return Err(AppError::conflict("A file with this name already exists"));
        }
    }

    let jpeg_path = if let Some(artwork_url) = artwork_url {
        Some(download_artwork_as_jpeg(&artwork_url, state.data_dir.join(".cache")).await?)
    } else {
        None
    };

    if let Some(jpeg_path) = jpeg_path {
        let embed_result = write_audio_cover(Path::new(&track.file_path), &jpeg_path).await;
        let _ = async_fs::remove_file(jpeg_path).await;
        embed_result?;
        track.has_cover =
            extract_cover(Path::new(&track.file_path), &state.covers_dir, track.id).await?;
    }

    let metadata_changed = new_title != track.title || new_artist != track.artist || rename_file;

    if new_title != track.title || new_artist != track.artist {
        write_audio_tags(Path::new(&track.file_path), &new_title, &new_artist).await?;
    }

    if rename_file && new_path != Path::new(&track.file_path) {
        move_without_replace(Path::new(&track.file_path), &new_path).await?;
    }

    track.title = new_title;
    track.artist = new_artist;
    track.file_path = new_path.to_string_lossy().to_string();
    if metadata_changed {
        track.updated_at = now();
    }
    let file_meta = fs::metadata(&new_path)?;
    track.inode = inode(&file_meta);
    track.file_signature = Some(file_signature(&file_meta));
    let hash_path = new_path.clone();
    track.content_hash = Some(
        tokio::task::spawn_blocking(move || file_content_hash(&hash_path))
            .await
            .map_err(|error| AppError::from(anyhow!(error)))??,
    );
    save_track(&state, &track)?;
    Ok(track_dto(track))
}

pub async fn delete_track(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> Result<StatusCode, AppError> {
    if !can_write(&state.music_dir) {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    if let Some(track) = get_track(&state, id)? {
        match fs::remove_file(&track.file_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        delete_track_row(&state, id)?;
        tracing::info!(track_id = id, "track deleted");
        broadcast(
            &state,
            "track_deleted",
            Some("Deleted track"),
            Some("info"),
            Some(json!({"id": id})),
        );
    }
    Ok(StatusCode::ACCEPTED)
}

pub async fn upload_track(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    if !can_write(&state.music_dir) {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    let mut title = None;
    let mut artist = None;
    let mut artwork_url = None;
    let mut file_name = None;
    let mut temp_path = None;
    while let Some(field) = multipart.next_field().await? {
        match field.name().unwrap_or_default() {
            "title" => title = Some(field.text().await?),
            "artist" => artist = Some(field.text().await?),
            "artwork_url" => artwork_url = Some(field.text().await?),
            "file" => {
                file_name = field.file_name().map(str::to_string);
                let source_name = file_name.as_deref().unwrap_or_default();
                let ext = Path::new(source_name)
                    .extension()
                    .and_then(|v| v.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if !matches!(ext.as_str(), "mp3" | "flac" | "wav") {
                    return Err(AppError::bad_request(
                        "Unsupported file format. Only MP3, FLAC, and WAV are supported.",
                    ));
                }

                let max_size_mb = env::var("MAX_UPLOAD_SIZE_MB")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(30);
                let max_bytes = max_size_mb * 1024 * 1024;
                let upload_dir = state.data_dir.join(".cache");
                async_fs::create_dir_all(&upload_dir).await?;
                let (path, mut file) = create_upload_temp_file(&upload_dir, &ext).await?;
                let mut written = 0usize;
                let mut field = field;
                while let Some(chunk) = field.chunk().await? {
                    written += chunk.len();
                    if written > max_bytes {
                        let _ = async_fs::remove_file(&path).await;
                        return Err(AppError {
                            status: StatusCode::PAYLOAD_TOO_LARGE,
                            message: format!(
                                "File size exceeds maximum allowed size ({max_size_mb}MB)"
                            ),
                        });
                    }
                    file.write_all(&chunk).await?;
                }
                file.flush().await?;
                temp_path = Some(path);
            }
            _ => {}
        }
    }
    let title = match title {
        Some(title) => title,
        None => {
            if let Some(temp_path) = temp_path {
                let _ = async_fs::remove_file(temp_path).await;
            }
            return Err(AppError::bad_request("Missing title"));
        }
    };
    let artist = match artist {
        Some(artist) => artist,
        None => {
            if let Some(temp_path) = temp_path {
                let _ = async_fs::remove_file(temp_path).await;
            }
            return Err(AppError::bad_request("Missing artist"));
        }
    };
    let source_name = match file_name {
        Some(file_name) => file_name,
        None => {
            if let Some(temp_path) = temp_path {
                let _ = async_fs::remove_file(temp_path).await;
            }
            return Err(AppError::bad_request("Missing file"));
        }
    };
    let ext = Path::new(&source_name)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let filename = generate_filename(&artist, &title, &ext)?;
    let path = state.music_dir.join(filename);
    if path.exists() {
        if let Some(temp_path) = temp_path {
            let _ = async_fs::remove_file(temp_path).await;
        }
        return Err(AppError::conflict("A file with this name already exists"));
    }
    let temp_path = temp_path.ok_or_else(|| AppError::bad_request("Missing file"))?;
    if let Err(error) = move_without_replace(&temp_path, &path).await {
        let _ = async_fs::remove_file(&temp_path).await;
        return Err(error);
    }
    if let Some(artwork_url) = artwork_url.filter(|url| !url.trim().is_empty()) {
        let jpeg_path =
            download_artwork_as_jpeg(&artwork_url, state.data_dir.join(".cache")).await?;
        let embed_result = write_audio_cover(&path, &jpeg_path).await;
        let _ = async_fs::remove_file(jpeg_path).await;
        embed_result?;
    }
    write_audio_tags(&path, &title, &artist).await?;
    let track = upsert_path(&state, &path, Some(title), Some(artist)).await?;
    tracing::info!(track_id = track.id, "track uploaded");
    Ok(Json(
        json!({"success": true, "message": "File uploaded and queued for processing."}),
    ))
}

async fn create_upload_temp_file(
    upload_dir: &Path,
    ext: &str,
) -> Result<(PathBuf, async_fs::File), AppError> {
    for attempt in 0..100 {
        let path = upload_dir.join(format!("upload-{}-{attempt}.{ext}", now_nanos()));
        match async_fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(AppError::from(anyhow::anyhow!(
        "failed to allocate upload file"
    )))
}

async fn move_without_replace(source: &Path, destination: &Path) -> Result<(), AppError> {
    let mut input = async_fs::File::open(source).await?;
    let mut output = async_fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .await
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                AppError::conflict("A file with this name already exists")
            } else {
                error.into()
            }
        })?;
    if let Err(error) = async_io::copy(&mut input, &mut output).await {
        let _ = async_fs::remove_file(destination).await;
        return Err(error.into());
    }
    if let Err(error) = output.flush().await {
        let _ = async_fs::remove_file(destination).await;
        return Err(error.into());
    }
    async_fs::remove_file(source).await?;
    Ok(())
}
