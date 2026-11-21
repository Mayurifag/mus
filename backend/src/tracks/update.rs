use std::path::Path;

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde_json::{json, Value};
use tokio::fs;

use crate::{
    artwork::download_artwork_as_jpeg,
    db::{self, get_track, save_track},
    derived::delete_track_derived_files,
    error::AppError,
    events::broadcast,
    files::{can_write, file_content_hash, file_signature, inode},
    media::{
        extract_cover, prepare_audio_update, read_cut_source, read_metadata, validate_cut,
        AudioUpdate,
    },
    models::{TrackDto, TrackUpdate},
    state::AppState,
    time::{now, system_time_secs},
};

use super::update_plan::UpdatePlan;

pub async fn update_track(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(update): Json<TrackUpdate>,
) -> Result<Json<Value>, AppError> {
    if !can_write(&state.music_dir).await {
        return Err(AppError::forbidden("Music directory is read-only"));
    }
    let track = get_track(&state, &id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    if let Some(cut) = update.remove_section {
        let source = read_cut_source(Path::new(&track.file_path)).await?;
        validate_cut(cut, source.duration)
            .map_err(|error| AppError::bad_request(&error.to_string()))?;
    }
    let plan = UpdatePlan::new(&state, &track, &update).await?;
    if !plan.changes_audio(&update) && !plan.tags_changed {
        return Ok(Json(json!({"status": "no_changes"})));
    }
    let rename_file = plan.rename_file;

    tokio::spawn(async move {
        match apply_track_update(&state, &id, update).await {
            Ok(dto) => {
                tracing::info!(track_id = dto.id, rename_file, "track updated");
                broadcast(
                    &state,
                    "track_updated",
                    Some("Track updated"),
                    Some("success"),
                    Some({
                        let mut payload = json!(dto);
                        payload["previous_id"] = json!(id);
                        payload
                    }),
                );
            }
            Err(error) => {
                let message = error.message;
                tracing::error!(
                    track_id = id.as_str(),
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
    state: &AppState,
    id: &str,
    update: TrackUpdate,
) -> Result<TrackDto, AppError> {
    let _guard = state.mutation_lock(format!("track:{id}")).await;
    let mut track = get_track(state, id)
        .await?
        .ok_or_else(|| AppError::not_found("Track not found"))?;
    let old_id = track.id.clone();
    let plan = UpdatePlan::new(state, &track, &update).await?;
    let only_tags_changed = !plan.changes_audio(&update) && plan.tags_changed;
    let UpdatePlan {
        title: new_title,
        artist: new_artist,
        path: mut new_path,
        tags: new_tags,
        metadata_changed,
        finalize_tags,
        rename_file,
        ..
    } = plan;
    let artwork_url = update.artwork_url.filter(|url| !url.trim().is_empty());
    let cache_dir = state.data_dir.join(".cache");
    if only_tags_changed {
        track.tags = db::set_track_tags(state, &track.id, &new_tags, finalize_tags).await?;
        return Ok(track.into());
    }

    let jpeg_path = if let Some(artwork_url) = artwork_url {
        Some(download_artwork_as_jpeg(&artwork_url, &cache_dir).await?)
    } else {
        None
    };

    let target_path =
        (rename_file && new_path != Path::new(&track.file_path)).then_some(new_path.as_path());
    let prepared = prepare_audio_update(
        Path::new(&track.file_path),
        &cache_dir,
        AudioUpdate {
            title: metadata_changed.then_some(new_title.as_str()),
            artist: metadata_changed.then_some(new_artist.as_str()),
            cover_jpeg_path: jpeg_path.as_deref(),
            target_path,
            standardize_tags: false,
            update_mtime: true,
            remove_section: update.remove_section,
        },
    )
    .await?;
    let content_hash = if let Some(path) = prepared.staged_path() {
        let path = path.to_path_buf();
        let hash = tokio::task::spawn_blocking(move || file_content_hash(&path))
            .await
            .map_err(anyhow::Error::from)??;
        if hash != old_id && get_track(state, &hash).await?.is_some() {
            return Err(AppError::conflict(
                "This edit would duplicate another track",
            ));
        }
        Some(hash)
    } else {
        None
    };
    let update_result = prepared.publish().await?;
    drop(jpeg_path);
    new_path = update_result.path;

    track.file_path = new_path.to_string_lossy().to_string();
    track.title = new_title;
    track.artist = new_artist;
    if update.remove_section.is_some() {
        track.duration = read_metadata(&new_path).await?.duration;
    }
    let file_meta = fs::metadata(&new_path).await?;
    if update_result.changed {
        track.updated_at = file_meta
            .modified()
            .ok()
            .and_then(system_time_secs)
            .unwrap_or_else(now);
    }
    track.inode = inode(&file_meta);
    track.file_signature = Some(file_signature(&file_meta));
    if let Some(content_hash) = content_hash {
        track.id = content_hash;
    }
    if old_id != track.id {
        delete_track_derived_files(state, &old_id).await;
    }
    if update_result.content_changed {
        track.has_cover = extract_cover(&new_path, &state.covers_dir, &track.id)
            .await
            .unwrap_or(false);
    }
    track.tags = save_track(state, &old_id, &track, &new_tags, finalize_tags).await?;
    Ok(track.into())
}
