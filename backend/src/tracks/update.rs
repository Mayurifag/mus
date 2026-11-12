use std::path::{Path, PathBuf};

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
    hls::prewarm_track_hls,
    media::{
        apply_audio_update, extract_cover, read_cut_source, read_metadata, validate_cut,
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
        db::set_track_tags(state, &track.id, &new_tags, finalize_tags).await?;
        return get_track(state, &track.id)
            .await?
            .map(TrackDto::from)
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
        let file_meta = fs::metadata(&new_path).await?;
        track.updated_at = file_meta
            .modified()
            .ok()
            .and_then(system_time_secs)
            .unwrap_or_else(now);
        track.inode = inode(&file_meta);
        track.file_signature = Some(file_signature(&file_meta));

        save_track(state, &old_id, &track).await?;
        db::set_track_tags(state, &track.id, &new_tags, finalize_tags).await?;
        track.tags = get_track(state, &track.id)
            .await?
            .map(|track| track.tags)
            .unwrap_or_default();

        if metadata_changed {
            spawn_audio_tag_sync(state.clone(), track.id.clone());
        }
        return Ok(track.into());
    }

    let jpeg_path = if let Some(artwork_url) = artwork_url {
        Some(download_artwork_as_jpeg(&artwork_url, &cache_dir).await?)
    } else {
        None
    };

    let target_path =
        (rename_file && new_path != Path::new(&track.file_path)).then_some(new_path.as_path());
    let update_result = apply_audio_update(
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
    if update_result.content_changed {
        let hash_path = new_path.clone();
        let content_hash = tokio::task::spawn_blocking(move || file_content_hash(&hash_path))
            .await
            .map_err(anyhow::Error::from)??;
        track.id = content_hash.clone();
        track.content_hash = content_hash;
    }
    if old_id != track.id {
        delete_track_derived_files(state, &old_id).await;
    }
    if update_result.content_changed {
        track.has_cover = extract_cover(&new_path, &state.covers_dir, &track.id)
            .await
            .unwrap_or(false);
    }
    save_track(state, &old_id, &track).await?;
    db::set_track_tags(state, &track.id, &new_tags, finalize_tags).await?;
    track.tags = get_track(state, &track.id)
        .await?
        .map(|track| track.tags)
        .unwrap_or_default();
    if update_result.content_changed {
        spawn_hls_prewarm(state.clone(), track.clone());
    }
    Ok(track.into())
}

fn spawn_hls_prewarm(state: AppState, track: crate::models::Track) {
    tokio::spawn(async move {
        prewarm_track_hls(&state, &track).await;
    });
}

fn spawn_audio_tag_sync(state: AppState, track_id: String) {
    tokio::spawn(async move {
        let _guard = state.mutation_lock(format!("track:{track_id}")).await;
        let Ok(Some(track)) = get_track(&state, &track_id).await else {
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
            Ok(_) => match fs::metadata(&path).await {
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
                    )
                    .await
                    {
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
