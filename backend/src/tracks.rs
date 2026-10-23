use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use axum::{
    body::Body,
    extract::{Multipart, Path as AxumPath, Query, State},
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::{fs as async_fs, io::AsyncWriteExt};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{
    artwork::download_artwork_as_jpeg,
    db::{self, delete_track_row, get_track, save_track, track_dto},
    derived::delete_track_derived_files,
    error::AppError,
    events::broadcast,
    hls::prewarm_track_hls,
    media::{
        apply_audio_update, extract_cover, preview_cut, read_cut_source, read_metadata,
        validate_cut, AudioUpdate,
    },
    models::{AudioCut, TrackDto, TrackUpdate},
    scanner::upsert_path,
    state::AppState,
    util::{
        can_write, file_content_hash, file_signature, generate_filename, inferred_tag_names, inode,
        is_hash_id, normalize_artists, normalize_text, now, now_nanos, system_time_secs,
    },
};

#[derive(Deserialize)]
pub struct TracksQuery {
    category: Option<String>,
}

pub async fn get_tracks(
    State(state): State<AppState>,
    Query(query): Query<TracksQuery>,
) -> Result<Json<Vec<TrackDto>>, AppError> {
    let category = query.category.as_deref().filter(|category| {
        matches!(
            *category,
            "right-version" | "ai-cover" | "cover" | "ost" | "mashup"
        )
    });
    let tracks = if let Some(category) = category {
        db::list_tracks_by_category(&state, category)?
    } else {
        db::list_tracks(&state)?
    };
    Ok(Json(tracks.into_iter().map(track_dto).collect()))
}

pub async fn get_cover_small(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Response, AppError> {
    cover_response(state, &id, "small").await
}

pub async fn get_cover_original(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Response, AppError> {
    cover_response(state, &id, "original").await
}

async fn cover_response(state: AppState, id: &str, size: &str) -> Result<Response, AppError> {
    if !is_hash_id(id) {
        return Err(AppError::not_found("Cover not found"));
    }
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
    AxumPath(id): AxumPath<String>,
    Json(update): Json<TrackUpdate>,
) -> Result<Json<Value>, AppError> {
    if !can_write(&state.music_dir) {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    let track = get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    if let Some(cut) = update.remove_section {
        let source = read_cut_source(Path::new(&track.file_path)).await?;
        validate_cut(cut, source.duration)
            .map_err(|error| AppError::bad_request(&error.to_string()))?;
    }
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

    let mut new_path = PathBuf::from(&track.file_path);
    if rename_file {
        let ext = new_path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("mp3");
        let filename = generate_filename(&new_artist, &new_title, ext)?;
        new_path = new_path.parent().unwrap_or(&state.music_dir).join(filename);
        if new_path != Path::new(&track.file_path)
            && path_exists_as_different_file(Path::new(&track.file_path), &new_path)?
        {
            return Err(AppError::conflict("A file with this name already exists"));
        }
    }
    let old_tags = track
        .tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect::<Vec<_>>();
    let new_tags = update.tags.clone().unwrap_or_else(|| {
        if update.remove_section.is_some() && new_title == track.title && new_artist == track.artist
        {
            old_tags.clone()
        } else {
            inferred_tag_names(&new_title, &new_artist, &new_path)
        }
    });
    if new_title == track.title
        && new_artist == track.artist
        && !rename_file
        && artwork_url.is_none()
        && update.remove_section.is_none()
        && new_tags == old_tags
    {
        return Ok(Json(json!({"status": "no_changes"})));
    }

    tokio::spawn(async move {
        let requested_id = id.clone();
        match apply_track_update(state.clone(), id, update).await {
            Ok(dto) => {
                tracing::info!(track_id = dto.id, rename_file, "track updated");
                broadcast(
                    &state,
                    "track_updated",
                    Some("Track updated"),
                    Some("success"),
                    Some({
                        let mut payload = json!(dto);
                        payload["previous_id"] = json!(requested_id);
                        payload
                    }),
                );
            }
            Err(error) => {
                let message = error.message;
                tracing::error!(
                    track_id = requested_id.as_str(),
                    error = message.as_str(),
                    "track update failed"
                );
                broadcast(
                    &state,
                    "track_update_failed",
                    Some("Failed to update track"),
                    Some("error"),
                    Some(json!({"id": requested_id, "error": message})),
                );
            }
        }
    });

    Ok(Json(json!({"status": "queued"})))
}

async fn apply_track_update(
    state: AppState,
    id: String,
    update: TrackUpdate,
) -> Result<TrackDto, AppError> {
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let mut track =
        get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let old_id = track.id.clone();
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
        if new_path != Path::new(&track.file_path)
            && path_exists_as_different_file(Path::new(&track.file_path), &new_path)?
        {
            return Err(AppError::conflict("A file with this name already exists"));
        }
    }
    let tags_changed = new_title != track.title || new_artist != track.artist;

    let old_tags = track
        .tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect::<Vec<_>>();
    let new_tags = update.tags.clone().unwrap_or_else(|| {
        if update.remove_section.is_some() && !tags_changed {
            old_tags.clone()
        } else {
            inferred_tag_names(&new_title, &new_artist, &new_path)
        }
    });
    let only_tags_changed = new_title == track.title
        && new_artist == track.artist
        && !rename_file
        && artwork_url.is_none()
        && update.remove_section.is_none()
        && new_tags != old_tags;
    if only_tags_changed {
        db::set_track_tags(&state, &track.id, &new_tags)?;
        return get_track(&state, &track.id)?
            .map(track_dto)
            .ok_or_else(|| AppError::not_found("Track not found"));
    }

    if artwork_url.is_none() && update.remove_section.is_none() {
        if rename_file && new_path != Path::new(&track.file_path) {
            let update_result = apply_audio_update(
                Path::new(&track.file_path),
                &cache_dir,
                AudioUpdate {
                    title: None,
                    artist: None,
                    cover_jpeg_path: None,
                    target_path: Some(new_path.as_path()),
                    standardize_tags: false,
                    update_mtime: true,
                    remove_section: None,
                },
            )
            .await?;
            new_path = update_result.path;
        }

        track.file_path = new_path.to_string_lossy().to_string();
        track.title = new_title;
        track.artist = new_artist;
        let file_meta = fs::metadata(&new_path)?;
        track.updated_at = file_meta
            .modified()
            .ok()
            .and_then(system_time_secs)
            .unwrap_or_else(now);
        track.inode = inode(&file_meta);
        track.file_signature = Some(file_signature(&file_meta));

        save_track(&state, &old_id, &track)?;
        db::set_track_tags(&state, &track.id, &new_tags)?;
        track.tags = get_track(&state, &track.id)?
            .map(|track| track.tags)
            .unwrap_or_default();

        if tags_changed {
            spawn_audio_tag_sync(state.clone(), track.clone());
        }
        return Ok(track_dto(track));
    }

    let jpeg_path = if let Some(artwork_url) = artwork_url {
        Some(download_artwork_as_jpeg(&artwork_url, cache_dir.clone()).await?)
    } else {
        None
    };

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
            remove_section: update.remove_section,
        },
    )
    .await;
    if let Some(jpeg_path) = jpeg_path {
        let _ = async_fs::remove_file(jpeg_path).await;
    }
    let update_result = update_result?;
    new_path = update_result.path;

    track.file_path = new_path.to_string_lossy().to_string();
    track.title = new_title;
    track.artist = new_artist;
    if update.remove_section.is_some() {
        track.duration = read_metadata(&new_path).await?.duration;
    }
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
    if update_result.content_changed {
        let hash_path = new_path.clone();
        let content_hash = tokio::task::spawn_blocking(move || file_content_hash(&hash_path))
            .await
            .map_err(|error| AppError::from(anyhow!(error)))??;
        track.id = content_hash.clone();
        track.content_hash = content_hash;
    }
    if old_id != track.id {
        delete_track_derived_files(&state, &old_id);
    }
    if update_result.content_changed {
        track.has_cover = extract_cover(&new_path, &state.covers_dir, &track.id)
            .await
            .unwrap_or(false);
    }
    save_track(&state, &old_id, &track)?;
    db::set_track_tags(&state, &track.id, &new_tags)?;
    track.tags = get_track(&state, &track.id)?
        .map(|track| track.tags)
        .unwrap_or_default();
    if update_result.content_changed {
        spawn_hls_prewarm(state.clone(), track.clone());
    }
    Ok(track_dto(track))
}

fn spawn_hls_prewarm(state: AppState, track: crate::models::Track) {
    tokio::spawn(async move {
        prewarm_track_hls(&state, &track).await;
    });
}

fn spawn_audio_tag_sync(state: AppState, track: crate::models::Track) {
    tokio::spawn(async move {
        let track_id = track.id.clone();
        let _guard = state.mutation_lock(format!("track:{track_id}")).await;
        let Ok(Some(track)) = get_track(&state, &track_id) else {
            return;
        };
        let path = PathBuf::from(&track.file_path);
        let result = apply_audio_update(
            &path,
            &state.data_dir.join(".cache"),
            AudioUpdate {
                title: Some(track.title.as_str()),
                artist: Some(track.artist.as_str()),
                cover_jpeg_path: None,
                target_path: None,
                standardize_tags: false,
                update_mtime: true,
                remove_section: None,
            },
        )
        .await;
        match result {
            Ok(_) => match fs::metadata(&path) {
                Ok(meta) => {
                    if let Err(error) = db::save_track_file_signature(
                        &state,
                        &track_id,
                        inode(&meta),
                        &file_signature(&meta),
                        meta.modified()
                            .ok()
                            .and_then(system_time_secs)
                            .unwrap_or_else(now),
                    ) {
                        tracing::warn!(track_id, error = %error, "failed to save audio tag sync signature");
                    }
                }
                Err(error) => {
                    tracing::warn!(track_id, error = %error, "failed to stat audio tag sync output");
                }
            },
            Err(error) => {
                tracing::warn!(track_id, error = %error, "failed to sync embedded audio tags");
            }
        }
    });
}

fn path_exists_as_different_file(source: &Path, destination: &Path) -> io::Result<bool> {
    let destination_meta = match fs::metadata(destination) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let source_meta = fs::metadata(source)?;
    Ok(!same_file_metadata(&source_meta, &destination_meta))
}

#[cfg(unix)]
fn same_file_metadata(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file_metadata(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len() && left.modified().ok() == right.modified().ok()
}

pub async fn get_cut_info(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, AppError> {
    get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let track = get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let source = read_cut_source(Path::new(&track.file_path)).await?;
    Ok(Json(json!({"duration_seconds": source.duration})))
}

#[derive(Deserialize)]
pub struct CutPreviewRequest {
    remove_section: AudioCut,
    #[serde(default)]
    selection_only: bool,
}

pub async fn get_cut_preview(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(request): Json<CutPreviewRequest>,
) -> Result<Response, AppError> {
    get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let track = get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    let path = Path::new(&track.file_path);
    let source = read_cut_source(path).await?;
    validate_cut(request.remove_section, source.duration)
        .map_err(|error| AppError::bad_request(&error.to_string()))?;
    let audio = preview_cut(
        path,
        request.remove_section,
        source.duration,
        request.selection_only,
    )
    .await?;
    Ok((
        [
            (header::CONTENT_TYPE, "audio/wav"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        audio,
    )
        .into_response())
}

pub async fn delete_track(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, AppError> {
    if !can_write(&state.music_dir) {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    if let Some(track) = get_track(&state, &id)? {
        match fs::remove_file(&track.file_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        delete_track_row(&state, &id)?;
        delete_track_derived_files(&state, &id);
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
            remove_section: None,
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
    let tags = inferred_tag_names(&track.title, &track.artist, &path);
    db::add_track_tags(&state, &track.id, &tags)?;
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
