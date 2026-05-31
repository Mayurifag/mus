use std::{fs, time::Duration};

use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    models::MusicDirectoryStatus,
    scanner::scan_music_dir,
    state::AppState,
    util::{can_write, command_output_text_with_timeout},
};

pub async fn get_permissions(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"can_write_music_files": can_write(&state.music_dir)}))
}

pub async fn get_system_info(State(state): State<AppState>) -> Json<Value> {
    let yt_dlp_version = state.yt_dlp_version.lock().unwrap().clone();
    Json(json!({
        "app_date": state.app_date,
        "commit_sha": state.commit_sha,
        "music_dir": music_directory_status(&state),
        "yt_dlp_version": yt_dlp_version,
    }))
}

fn music_directory_status(state: &AppState) -> MusicDirectoryStatus {
    let exists = state.music_dir.exists();
    let is_directory = state.music_dir.is_dir();
    let is_empty = is_directory.then(|| {
        fs::read_dir(&state.music_dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
    });
    let warning = if !exists {
        Some("Music directory is missing. Check the /app_data/music mount.".to_string())
    } else if !is_directory {
        Some("Music path exists but is not a directory.".to_string())
    } else {
        None
    };

    MusicDirectoryStatus {
        path: state.music_dir.to_string_lossy().to_string(),
        exists,
        is_directory,
        is_empty,
        can_write: can_write(&state.music_dir),
        warning,
    }
}

pub async fn update_yt_dlp(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    tracing::info!("yt-dlp update started");
    let output = command_output_text_with_timeout(
        "yt-dlp",
        &["--update-to", "nightly"],
        Duration::from_secs(300),
    )
    .await?;
    let version =
        command_output_text_with_timeout("yt-dlp", &["--version"], Duration::from_secs(10))
            .await
            .ok();
    *state.yt_dlp_version.lock().unwrap() = version.clone();
    tracing::info!(
        yt_dlp_version = version.as_deref().unwrap_or("unknown"),
        "yt-dlp update completed"
    );
    Ok(Json(json!({"yt_dlp_version": version, "output": output})))
}

pub async fn rescan(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    scan_music_dir(state).await?;
    Ok(Json(json!({"status": "ok"})))
}
