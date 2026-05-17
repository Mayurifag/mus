use std::{collections::HashMap, collections::HashSet, fs, path::Path, time::Duration};

use anyhow::Result;
use rusqlite::params;
use serde_json::json;
use tokio::time;

use crate::{
    db::{self, delete_track_row, set_track_cover_state, track_dto},
    events::broadcast,
    media::{extract_cover, read_metadata, standardize_audio_tags, MediaMetadata},
    state::AppState,
    util::{audio_paths, inode, now, system_time_secs},
};

pub async fn scan_music_dir(state: AppState) -> Result<()> {
    let paths = audio_paths(&state.music_dir)?;
    tracing::info!("found {} audio files", paths.len());
    let mut seen = HashSet::new();
    for path in paths {
        seen.insert(path.to_string_lossy().to_string());
        if let Err(error) = upsert_path(&state, &path, None, None).await {
            tracing::warn!("failed to scan {}: {error}", path.display());
        }
    }
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id,file_path FROM track")?;
    let stale = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    for (id, path) in stale {
        if !seen.contains(&path) && !Path::new(&path).exists() {
            conn.execute("DELETE FROM track WHERE id = ?1", params![id])?;
        }
    }
    Ok(())
}

pub async fn watch_music_dir(state: AppState) {
    let mut interval = time::interval(Duration::from_secs(2));
    loop {
        interval.tick().await;
        if let Err(error) = sync_music_dir(state.clone()).await {
            tracing::warn!("failed to sync music directory: {error}");
        }
    }
}

async fn sync_music_dir(state: AppState) -> Result<()> {
    let paths = audio_paths(&state.music_dir)?;
    let mut current = HashSet::new();
    let mut existing = HashMap::new();
    for track in db::list_tracks(&state)? {
        existing.insert(track.file_path.clone(), track);
    }

    for path in paths {
        let path_string = path.to_string_lossy().to_string();
        current.insert(path_string.clone());
        let file_mtime = fs::metadata(&path)
            .ok()
            .and_then(|v| v.modified().ok())
            .and_then(system_time_secs)
            .unwrap_or_else(now);
        match existing.get(&path_string) {
            Some(track) if track.updated_at >= file_mtime => {}
            Some(_) => {
                let track = upsert_path(&state, &path, None, None).await?;
                broadcast(
                    &state,
                    "track_updated",
                    None,
                    Some("info"),
                    Some(json!(track)),
                );
            }
            None => {
                let track = upsert_path(&state, &path, None, None).await?;
                broadcast(
                    &state,
                    "track_added",
                    Some(&format!("Added track '{}'", track.title)),
                    Some("success"),
                    Some(json!(track)),
                );
            }
        }
    }

    for track in existing.values() {
        if !current.contains(&track.file_path) && !Path::new(&track.file_path).exists() {
            delete_track_row(&state, track.id)?;
            broadcast(
                &state,
                "track_deleted",
                Some(&format!("Deleted track '{}'", track.title)),
                Some("info"),
                Some(json!({"id": track.id})),
            );
        }
    }
    Ok(())
}

pub async fn upsert_path(
    state: &AppState,
    path: &Path,
    title_override: Option<String>,
    artist_override: Option<String>,
) -> Result<crate::models::TrackDto> {
    if let Err(error) = standardize_audio_tags(path).await {
        tracing::warn!("failed to standardize tags for {}: {error}", path.display());
    }
    let metadata = read_metadata(path).await.unwrap_or_else(|_| MediaMetadata {
        title: path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or("Unknown Title")
            .to_string(),
        artist: "Unknown Artist".into(),
        duration: 0,
    });
    let file_meta = fs::metadata(path)?;
    let added_at = file_meta
        .modified()
        .ok()
        .and_then(system_time_secs)
        .unwrap_or_else(now);
    let title = title_override.unwrap_or(metadata.title);
    let artist = artist_override.unwrap_or(metadata.artist);
    let file_path = path.to_string_lossy().to_string();
    let (id, added_at) = {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (title, artist, duration, file_path, added_at, updated_at, has_cover, inode, processing_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0, ?6, 'COMPLETE')
             ON CONFLICT(file_path) DO UPDATE SET title=excluded.title, artist=excluded.artist, duration=excluded.duration, updated_at=excluded.updated_at, processing_status='COMPLETE', last_error=NULL",
            params![title, artist, metadata.duration, file_path, added_at, inode(&file_meta)],
        )?;
        let id = conn.query_row(
            "SELECT id FROM track WHERE file_path = ?1",
            params![path.to_string_lossy().to_string()],
            |row| row.get::<_, i64>(0),
        )?;
        (id, added_at)
    };
    let has_cover = extract_cover(path, &state.covers_dir, id)
        .await
        .unwrap_or(false);
    set_track_cover_state(state, id, has_cover)?;
    Ok(track_dto(crate::models::Track {
        id,
        title,
        artist,
        duration: metadata.duration,
        file_path,
        added_at,
        updated_at: added_at,
        has_cover,
        inode: inode(&file_meta),
        content_hash: None,
        processing_status: "COMPLETE".into(),
        last_error: None,
    }))
}
