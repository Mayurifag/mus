use std::path::Path;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde_json::{json, Value};

use crate::{
    db::{errored_tracks, get_track, track_dto},
    error::AppError,
    models::TrackDto,
    scanner::upsert_path,
    state::AppState,
};

pub async fn get_errored_tracks(
    State(state): State<AppState>,
) -> Result<Json<Vec<TrackDto>>, AppError> {
    let tracks = errored_tracks(&state)?;
    Ok(Json(tracks.into_iter().map(track_dto).collect()))
}

pub async fn requeue_track(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, AppError> {
    let track = get_track(&state, &id)?.ok_or_else(|| AppError::not_found("Track not found"))?;
    upsert_path(&state, Path::new(&track.file_path), None, None).await?;
    Ok(Json(
        json!({"message": format!("Track '{}' re-queued for processing", track.title)}),
    ))
}
