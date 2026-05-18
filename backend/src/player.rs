use axum::{extract::State, Json};

use crate::{db, error::AppError, models::PlayerState, state::AppState};

pub async fn get_player_state(
    State(state): State<AppState>,
) -> Result<Json<PlayerState>, AppError> {
    Ok(Json(db::load_player_state(&state)?))
}

pub async fn save_player_state(
    State(state): State<AppState>,
    Json(player_state): Json<PlayerState>,
) -> Result<Json<PlayerState>, AppError> {
    db::save_player_state(&state, &player_state)?;
    Ok(Json(player_state))
}
