use std::{
    collections::HashMap,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use notify::{event::EventKind, Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::params;
use serde_json::json;
use tokio::{sync::mpsc, time};

use crate::{
    db::{self, delete_track_row, set_track_cover_state, track_dto},
    events::broadcast,
    media::{extract_cover, read_metadata, standardize_audio_tags, MediaMetadata},
    state::AppState,
    util::{file_content_hash, file_signature, inode, is_audio_path, now, system_time_secs},
};

struct FileSnapshot {
    meta: fs::Metadata,
    mtime: i64,
    file_signature: String,
    content_hash: String,
}

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
        let event = match event {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!("failed to receive music directory event: {error}");
                continue;
            }
        };
        if !should_sync_event(&event) {
            continue;
        }
        let mut paths = event.paths;
        time::sleep(Duration::from_millis(500)).await;
        while let Ok(event) = rx.try_recv() {
            match event {
                Ok(event) if should_sync_event(&event) => paths.extend(event.paths),
                Ok(_) => {}
                Err(error) => tracing::warn!("failed to receive music directory event: {error}"),
            }
        }
        if let Err(error) = sync_changed_paths(state.clone(), paths).await {
            tracing::warn!("failed to sync music directory: {error}");
        }
    }
}

async fn sync_changed_paths(state: AppState, paths: Vec<PathBuf>) -> Result<()> {
    let mut current = HashSet::new();
    let mut existing = HashMap::new();
    for track in db::list_tracks(&state)? {
        existing.insert(track.file_path.clone(), track);
    }

    let mut removed_paths = Vec::new();
    for path in paths {
        if path.is_dir() {
            for path in audio_paths_in_dir(&path)? {
                sync_path(&state, &existing, &mut current, &path, true).await?;
            }
        } else if path.exists() && is_audio_path(&path) {
            sync_path(&state, &existing, &mut current, &path, true).await?;
        } else if is_audio_path(&path) {
            removed_paths.push(path);
        }
    }
    for path in removed_paths {
        delete_track_path(&state, &path)?;
    }
    Ok(())
}

fn audio_paths_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                dirs.push(path);
            } else if is_audio_path(&path) {
                paths.push(path);
            }
        }
    }
    Ok(paths)
}

fn should_sync_event(event: &Event) -> bool {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
        _ => return false,
    }
    event
        .paths
        .iter()
        .any(|path| is_audio_path(path) || path.is_dir())
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
    let (meta, mtime, signature) = file_metadata(path)?;
    match existing.get(&path_string) {
        Some(track)
            if track.file_signature.as_deref() == Some(signature.as_str())
                && track.content_hash.is_some() => {}
        Some(track) => {
            let snapshot = file_snapshot_from_metadata(path, meta, mtime, signature).await?;
            if track.content_hash.is_none() && track.updated_at >= snapshot.mtime {
                db::save_track_fingerprint(
                    state,
                    track.id,
                    inode(&snapshot.meta),
                    &snapshot.file_signature,
                    &snapshot.content_hash,
                )?;
                return Ok(());
            }
            if track.content_hash.as_deref() == Some(snapshot.content_hash.as_str()) {
                db::save_track_fingerprint(
                    state,
                    track.id,
                    inode(&snapshot.meta),
                    &snapshot.file_signature,
                    &snapshot.content_hash,
                )?;
                return Ok(());
            }
            let track = upsert_path_with_snapshot(state, path, None, None, snapshot, None).await?;
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
            let snapshot = file_snapshot_from_metadata(path, meta, mtime, signature).await?;
            let moved_track = snapshot
                .meta
                .is_file()
                .then(|| {
                    existing.values().find(|track| {
                        track.inode == inode(&snapshot.meta)
                            && track.inode.is_some()
                            && !Path::new(&track.file_path).exists()
                    })
                })
                .flatten();
            if let Some(track) = moved_track {
                current.insert(track.file_path.clone());
            }
            let track = upsert_path_with_snapshot(
                state,
                path,
                moved_track.map(|track| track.title.clone()),
                moved_track.map(|track| track.artist.clone()),
                snapshot,
                moved_track.map(|track| track.id),
            )
            .await?;
            if broadcast_changes {
                if moved_track.is_some() {
                    broadcast(
                        state,
                        "track_updated",
                        None,
                        Some("info"),
                        Some(json!(track)),
                    );
                } else {
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
    }
    Ok(())
}

fn file_metadata(path: &Path) -> Result<(fs::Metadata, i64, String)> {
    let meta = fs::metadata(path)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(system_time_secs)
        .unwrap_or_else(now);
    let signature = file_signature(&meta);
    Ok((meta, mtime, signature))
}

async fn file_snapshot(path: &Path) -> Result<FileSnapshot> {
    let (meta, mtime, signature) = file_metadata(path)?;
    file_snapshot_from_metadata(path, meta, mtime, signature).await
}

async fn file_snapshot_from_metadata(
    path: &Path,
    meta: fs::Metadata,
    mtime: i64,
    file_signature: String,
) -> Result<FileSnapshot> {
    let path = path.to_path_buf();
    let content_hash = tokio::task::spawn_blocking(move || file_content_hash(&path)).await??;
    Ok(FileSnapshot {
        meta,
        mtime,
        file_signature,
        content_hash,
    })
}

pub async fn upsert_path(
    state: &AppState,
    path: &Path,
    title_override: Option<String>,
    artist_override: Option<String>,
) -> Result<crate::models::TrackDto> {
    let snapshot = file_snapshot(path).await?;
    upsert_path_with_snapshot(state, path, title_override, artist_override, snapshot, None).await
}

async fn upsert_path_with_snapshot(
    state: &AppState,
    path: &Path,
    title_override: Option<String>,
    artist_override: Option<String>,
    _snapshot: FileSnapshot,
    existing_id: Option<i64>,
) -> Result<crate::models::TrackDto> {
    standardize_audio_tags(path).await?;
    let snapshot = file_snapshot(path).await?;
    let metadata = read_metadata(path).await.unwrap_or_else(|_| MediaMetadata {
        title: path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or("Unknown Title")
            .to_string(),
        artist: "Unknown Artist".into(),
        duration: 0,
    });
    let added_at = snapshot.mtime;
    let title = title_override.unwrap_or(metadata.title);
    let artist = artist_override.unwrap_or(metadata.artist);
    let file_path = path.to_string_lossy().to_string();
    let (id, added_at) = {
        let conn = state.db.lock().unwrap();
        let id = if let Some(id) = existing_id {
            conn.query_row(
                "UPDATE track SET title=?1, artist=?2, duration=?3, file_path=?4, updated_at=?5, inode=?6, file_signature=?7, content_hash=?8, processing_status='COMPLETE', last_error=NULL WHERE id=?9 RETURNING id",
                params![&title, &artist, metadata.duration, &file_path, added_at, inode(&snapshot.meta), &snapshot.file_signature, &snapshot.content_hash, id],
                |row| row.get::<_, i64>(0),
            )?
        } else {
            conn.query_row(
                "INSERT INTO track (title, artist, duration, file_path, added_at, updated_at, has_cover, inode, file_signature, content_hash, processing_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0, ?6, ?7, ?8, 'COMPLETE')
                 ON CONFLICT(file_path) DO UPDATE SET title=excluded.title, artist=excluded.artist, duration=excluded.duration, updated_at=excluded.updated_at, inode=excluded.inode, file_signature=excluded.file_signature, content_hash=excluded.content_hash, processing_status='COMPLETE', last_error=NULL
                 RETURNING id",
                params![&title, &artist, metadata.duration, &file_path, added_at, inode(&snapshot.meta), &snapshot.file_signature, &snapshot.content_hash],
                |row| row.get::<_, i64>(0),
            )?
        };
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
        inode: inode(&snapshot.meta),
        file_signature: Some(snapshot.file_signature),
        content_hash: Some(snapshot.content_hash),
        processing_status: "COMPLETE".into(),
        last_error: None,
    }))
}

fn delete_track_path(state: &AppState, path: &Path) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "DELETE FROM track WHERE file_path = ?1",
        params![path.to_string_lossy().to_string()],
    )?;
    Ok(())
}
