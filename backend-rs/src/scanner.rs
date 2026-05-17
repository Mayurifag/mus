use std::{collections::HashMap, collections::HashSet, fs, path::Path, time::Duration};

use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::params;
use serde_json::json;
use tokio::{sync::mpsc, time};

use crate::{
    db::{self, delete_track_row, set_track_cover_state, track_dto},
    events::broadcast,
    media::{extract_cover, read_metadata, standardize_audio_tags, MediaMetadata},
    state::AppState,
    util::{inode, is_audio_path, now, system_time_secs},
};

pub async fn scan_music_dir(state: AppState) -> Result<()> {
    sync_tracks(state, false).await
}

pub async fn watch_music_dir(state: AppState) {
    let (tx, mut rx) = mpsc::channel(100);
    let mut watcher = match RecommendedWatcher::new(
        move |result| {
            let _ = tx.blocking_send(result);
        },
        Config::default(),
    ) {
        Ok(watcher) => watcher,
        Err(error) => {
            tracing::warn!("failed to start music directory watcher: {error}");
            return;
        }
    };
    if let Err(error) = watcher.watch(&state.music_dir, RecursiveMode::Recursive) {
        tracing::warn!("failed to watch music directory: {error}");
        return;
    }

    loop {
        let Some(event) = rx.recv().await else {
            break;
        };
        if let Err(error) = event {
            tracing::warn!("failed to receive music directory event: {error}");
            continue;
        }
        time::sleep(Duration::from_millis(500)).await;
        while rx.try_recv().is_ok() {}
        if let Err(error) = sync_tracks(state.clone(), true).await {
            tracing::warn!("failed to sync music directory: {error}");
        }
    }
}

async fn sync_tracks(state: AppState, broadcast_changes: bool) -> Result<()> {
    let mut current = HashSet::new();
    let mut existing = HashMap::new();
    for track in db::list_tracks(&state)? {
        existing.insert(track.file_path.clone(), track);
    }

    let mut dirs = vec![state.music_dir.clone()];
    let mut found = 0usize;
    while let Some(dir) = dirs.pop() {
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if !is_audio_path(&path) {
                continue;
            }

            found += 1;
            sync_path(&state, &existing, &mut current, &path, broadcast_changes).await?;
        }
    }
    tracing::info!("found {found} audio files");

    for track in existing.values() {
        if !current.contains(&track.file_path) && !Path::new(&track.file_path).exists() {
            delete_track_row(&state, track.id)?;
            if broadcast_changes {
                broadcast(
                    &state,
                    "track_deleted",
                    Some(&format!("Deleted track '{}'", track.title)),
                    Some("info"),
                    Some(json!({"id": track.id})),
                );
            }
        }
    }
    Ok(())
}

async fn sync_path(
    state: &AppState,
    existing: &HashMap<String, crate::models::Track>,
    current: &mut HashSet<String>,
    path: &Path,
    broadcast_changes: bool,
) -> Result<()> {
    let path_string = path.to_string_lossy().to_string();
    current.insert(path_string.clone());
    let file_mtime = file_mtime(path);
    match existing.get(&path_string) {
        Some(track) if track.updated_at >= file_mtime => {}
        Some(_) => {
            let track = upsert_path(state, path, None, None).await?;
            if broadcast_changes {
                broadcast(
                    state,
                    "track_updated",
                    None,
                    Some("info"),
                    Some(json!(track)),
                );
            }
        }
        None => {
            let track = upsert_path(state, path, None, None).await?;
            if broadcast_changes {
                broadcast(
                    state,
                    "track_added",
                    Some(&format!("Added track '{}'", track.title)),
                    Some("success"),
                    Some(json!(track)),
                );
            }
        }
    }
    Ok(())
}

fn file_mtime(path: &Path) -> i64 {
    fs::metadata(path)
        .ok()
        .and_then(|v| v.modified().ok())
        .and_then(system_time_secs)
        .unwrap_or_else(now)
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
