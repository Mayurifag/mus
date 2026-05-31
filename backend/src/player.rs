use axum::{extract::State, Json};

use crate::{db, error::AppError, models::PlayerState, state::AppState, util::is_hash_id};

pub async fn get_player_state(
    State(state): State<AppState>,
) -> Result<Json<PlayerState>, AppError> {
    Ok(Json(db::load_player_state(&state)?))
}

pub async fn save_player_state(
    State(state): State<AppState>,
    Json(player_state): Json<PlayerState>,
) -> Result<Json<PlayerState>, AppError> {
    if player_state
        .current_track_id
        .as_deref()
        .is_some_and(|id| !is_hash_id(id))
    {
        return Err(AppError::bad_request("Invalid track id"));
    }
    db::save_player_state(&state, &player_state)?;
    Ok(Json(player_state))
}
