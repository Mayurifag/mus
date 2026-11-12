use std::{io, path::Path};

use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::fs;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::{
    categories::category_display_name,
    db::{self, delete_track_row, get_track},
    derived::delete_track_derived_files,
    error::AppError,
    events::broadcast,
    files::{can_write, is_hash_id},
    media::{preview_cut, read_cut_source, validate_cut, CutPreview},
    models::{AudioCut, TrackDto},
    state::AppState,
};

mod failed;
mod update;
mod update_plan;
mod upload;
pub use failed::{get_errored_tracks, requeue_track};
pub use update::update_track;
pub use upload::upload_track;

#[derive(Deserialize)]
pub struct TracksQuery {
    category: Option<String>,
}

pub async fn get_tracks(
    State(state): State<AppState>,
    Query(query): Query<TracksQuery>,
) -> Result<Json<Vec<TrackDto>>, AppError> {
    let category = query
        .category
        .as_deref()
        .filter(|category| category_display_name(category).is_some());
    let tracks = if let Some(category) = category {
        db::list_tracks_by_category(&state, category).await?
    } else {
        db::list_tracks(&state).await?
    };
    Ok(Json(tracks.into_iter().map(TrackDto::from).collect()))
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
    if !fs::metadata(&cover_path)
        .await
        .is_ok_and(|meta| meta.is_file())
    {
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
        match ServeFile::new(cover_path)
            .oneshot(Request::new(Body::empty()))
            .await
        {
            Ok(response) => response,
            Err(error) => match error {},
        },
    )
        .into_response())
}

pub async fn get_cut_info(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, AppError> {
    get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let track = get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
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
    get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let track = get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    let path = Path::new(&track.file_path);
    let source = read_cut_source(path).await?;
    validate_cut(request.remove_section, source.duration)
        .map_err(|error| AppError::bad_request(&error.to_string()))?;
    let audio = preview_cut(
        path,
        request.remove_section,
        source.duration,
        if request.selection_only {
            CutPreview::Selection
        } else {
            CutPreview::Join
        },
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
    if !can_write(&state.music_dir).await {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let track = get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found; refresh and try again"))?;
    match fs::remove_file(&track.file_path).await {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    delete_track_row(&state, &id).await?;
    delete_track_derived_files(&state, &id).await;
    tracing::info!(track_id = id, "track deleted");
    broadcast(
        &state,
        "track_deleted",
        Some("Deleted track"),
        Some("info"),
        Some(json!({"id": id})),
    );
    Ok(StatusCode::ACCEPTED)
}
