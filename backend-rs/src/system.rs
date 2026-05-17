use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    scanner::scan_music_dir,
    state::AppState,
    util::{can_write, command_output},
};

pub async fn get_permissions(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"can_write_music_files": can_write(&state.music_dir)}))
}

pub async fn get_system_info(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "app_date": state.app_date,
        "commit_sha": state.commit_sha,
        "yt_dlp_version": command_output("yt-dlp", &["--version"]).await.ok(),
    }))
}

pub async fn update_yt_dlp() -> Result<Json<Value>, AppError> {
    let output = command_output("yt-dlp", &["--update-to", "nightly"]).await?;
    let version = command_output("yt-dlp", &["--version"]).await.ok();
    Ok(Json(json!({"yt_dlp_version": version, "output": output})))
}

pub async fn rescan(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    scan_music_dir(state).await?;
    Ok(Json(json!({"status": "ok"})))
}
