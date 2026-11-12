use axum::{extract::State, Json};

use crate::{db, error::AppError, files::is_hash_id, models::PlayerState, state::AppState};

pub async fn get_player_state(
    State(state): State<AppState>,
) -> Result<Json<PlayerState>, AppError> {
    Ok(Json(db::load_player_state(&state).await?))
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
    if !(0.0..=1.0).contains(&player_state.volume_level) {
        return Err(AppError::bad_request("Volume must be between 0 and 1"));
    }
    if !player_state.progress_seconds.is_finite() || player_state.progress_seconds < 0.0 {
        return Err(AppError::bad_request(
            "Progress must be finite and nonnegative",
        ));
    }
    db::save_player_state(&state, &player_state).await?;
    Ok(Json(player_state))
}
