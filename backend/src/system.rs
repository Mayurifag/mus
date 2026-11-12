use std::time::Duration;
use tokio::fs;

use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::{
    error::AppError, events, files::can_write, models::MusicDirectoryStatus,
    process::command_output_text_with_timeout, scanner::scan_music_dir, state::AppState,
};

pub async fn get_permissions(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"can_write_music_files": can_write(&state.music_dir).await}))
}

pub async fn get_system_info(State(state): State<AppState>) -> Json<Value> {
    let yt_dlp_version = state.yt_dlp_version.lock().await.clone();
    Json(json!({
        "app_date": state.app_date,
        "commit_sha": state.commit_sha,
        "music_dir": music_directory_status(&state).await,
        "yt_dlp_version": yt_dlp_version,
    }))
}

async fn music_directory_status(state: &AppState) -> MusicDirectoryStatus {
    let metadata = fs::metadata(&state.music_dir).await;
    let exists = metadata.is_ok();
    let is_directory = metadata.is_ok_and(|meta| meta.is_dir());
    let is_empty = if is_directory {
        Some(match fs::read_dir(&state.music_dir).await {
            Ok(mut entries) => matches!(entries.next_entry().await, Ok(None)),
            Err(_) => false,
        })
    } else {
        None
    };
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
        can_write: can_write(&state.music_dir).await,
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
    *state.yt_dlp_version.lock().await = version.clone();
    events::broadcast(
        &state,
        "yt_dlp_updated",
        None,
        None,
        Some(json!({ "yt_dlp_version": version })),
    );
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
