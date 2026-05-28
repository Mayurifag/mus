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
use tokio::{fs as async_fs, io::AsyncWriteExt};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{
    artwork::download_artwork_as_jpeg,
    db::{self, delete_track_row, get_track, save_track, track_dto},
    error::AppError,
    events::broadcast,
    media::{apply_audio_update, extract_cover, AudioUpdate},
    models::{ShuffleNextRequest, TrackDto, TrackUpdate},
    prewarm::prewarm_track_path,
    scanner::upsert_path,
    state::AppState,
    util::{
        can_write, file_content_hash, file_signature, generate_filename, inode, normalize_artists,
        normalize_text, now, now_nanos, parse_artists, system_time_secs,
    },
};

pub async fn get_tracks(State(state): State<AppState>) -> Result<Json<Vec<TrackDto>>, AppError> {
    let tracks = db::list_tracks(&state)?;
    Ok(Json(tracks.into_iter().map(track_dto).collect()))
}

pub async fn get_shuffle_next(
    State(state): State<AppState>,
    Json(request): Json<ShuffleNextRequest>,
) -> Result<Json<Option<TrackDto>>, AppError> {
    let selected_artist = request
        .selected_artist
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut candidates = db::list_tracks(&state)?
        .into_iter()
        .filter(|track| Some(track.id) != request.current_track_id)
        .filter(|track| Path::new(&track.file_path).is_file())
        .filter(|track| {
            selected_artist.is_none_or(|artist| {
                parse_artists(&track.artist)
                    .iter()
                    .any(|value| value == artist)
            })
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Ok(Json(None));
    }

    let index = (now_nanos() % candidates.len() as u128) as usize;
    let track = candidates.swap_remove(index);
    if let Err(error) = prewarm_track_path(Path::new(&track.file_path)).await {
        tracing::warn!(track_id = track.id, path = %track.file_path, "failed to prewarm shuffle track: {error}");
    }

    Ok(Json(Some(track_dto(track))))
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
    let file_size = fs::metadata(&track.file_path)?.len();
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
    normalize_suffix_range(request.headers_mut(), file_size);
    Ok((
        headers,
        ServeFile::new(track.file_path)
            .oneshot(request)
            .await
            .unwrap(),
    )
        .into_response())
}

fn normalize_suffix_range(headers: &mut HeaderMap, file_size: u64) {
    if file_size == 0 {
        return;
    }
    let Some(range) = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    else {
        return;
    };
    let Some(suffix) = range.strip_prefix("bytes=-") else {
        return;
    };
    let Ok(bytes) = suffix.parse::<u64>() else {
        return;
    };
    if bytes >= file_size {
        let value = format!("bytes=0-{}", file_size - 1);
        if let Ok(value) = HeaderValue::from_str(&value) {
            headers.insert(header::RANGE, value);
        }
    }
}

pub async fn prewarm_track(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> Result<StatusCode, AppError> {
    let track = get_track(&state, id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    if !Path::new(&track.file_path).is_file() {
        return Err(AppError::not_found("Audio file not found"));
    }
    if let Err(error) = prewarm_track_path(Path::new(&track.file_path)).await {
        tracing::warn!(track_id = track.id, path = %track.file_path, "failed to prewarm track: {error}");
    }

    Ok(StatusCode::NO_CONTENT)
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
    let new_title = normalize_text(&update.title.clone().unwrap_or_else(|| track.title.clone()));
    let new_artist = normalize_artists(
        &update
            .artist
            .clone()
            .unwrap_or_else(|| track.artist.clone()),
    );
    let rename_file = update.rename_file.unwrap_or(false);
    let artwork_url = update
        .artwork_url
        .as_deref()
        .filter(|url| !url.trim().is_empty());

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
        match apply_track_update(state.clone(), id, update).await {
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
    update: TrackUpdate,
) -> Result<TrackDto, AppError> {
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let mut track = get_track(&state, id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let new_title = normalize_text(&update.title.unwrap_or_else(|| track.title.clone()));
    let new_artist = normalize_artists(&update.artist.unwrap_or_else(|| track.artist.clone()));
    let rename_file = update.rename_file.unwrap_or(false);
    let artwork_url = update.artwork_url.filter(|url| !url.trim().is_empty());
    let cache_dir = state.data_dir.join(".cache");
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
        Some(download_artwork_as_jpeg(&artwork_url, cache_dir.clone()).await?)
    } else {
        None
    };

    let tags_changed = new_title != track.title || new_artist != track.artist;
    let target_path =
        (rename_file && new_path != Path::new(&track.file_path)).then_some(new_path.as_path());
    let update_result = apply_audio_update(
        Path::new(&track.file_path),
        &cache_dir,
        AudioUpdate {
            title: tags_changed.then_some(new_title.as_str()),
            artist: tags_changed.then_some(new_artist.as_str()),
            cover_jpeg_path: jpeg_path.as_deref(),
            target_path,
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await;
    if let Some(jpeg_path) = jpeg_path {
        let _ = async_fs::remove_file(jpeg_path).await;
    }
    let update_result = update_result?;
    new_path = update_result.path;

    track.file_path = new_path.to_string_lossy().to_string();
    if update_result.changed {
        track.has_cover = extract_cover(&new_path, &state.covers_dir, track.id)
            .await
            .unwrap_or(false);
    }

    track.title = new_title;
    track.artist = new_artist;
    let file_meta = fs::metadata(&new_path)?;
    if update_result.changed {
        track.updated_at = file_meta
            .modified()
            .ok()
            .and_then(system_time_secs)
            .unwrap_or_else(now);
    }
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
    let cache_dir = state.data_dir.join(".cache");
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
                async_fs::create_dir_all(&cache_dir).await?;
                let (path, mut file) = create_upload_temp_file(&cache_dir, &ext).await?;
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
        Some(title) => normalize_text(&title),
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
    let artist = normalize_artists(&artist);
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
    let jpeg_path = if let Some(artwork_url) = artwork_url.filter(|url| !url.trim().is_empty()) {
        match download_artwork_as_jpeg(&artwork_url, cache_dir.clone()).await {
            Ok(jpeg_path) => Some(jpeg_path),
            Err(error) => {
                let _ = async_fs::remove_file(&temp_path).await;
                return Err(error.into());
            }
        }
    } else {
        None
    };
    let update_result = apply_audio_update(
        &temp_path,
        &cache_dir,
        AudioUpdate {
            title: Some(&title),
            artist: Some(&artist),
            cover_jpeg_path: jpeg_path.as_deref(),
            target_path: Some(&path),
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await;
    if let Some(jpeg_path) = jpeg_path {
        let _ = async_fs::remove_file(jpeg_path).await;
    }
    if let Err(error) = update_result {
        let _ = async_fs::remove_file(&temp_path).await;
        return Err(error.into());
    }
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
