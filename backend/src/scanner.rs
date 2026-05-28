use std::{
    collections::HashMap,
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use notify::{
    event::EventKind, Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher,
};
use rusqlite::params;
use serde_json::json;
use tokio::{sync::mpsc, time};

use crate::{
    db::{self, delete_track_row, set_track_cover_state, track_dto},
    events::broadcast,
    media::{apply_audio_update, extract_cover, read_metadata, AudioUpdate, MediaMetadata},
    state::AppState,
    util::{
        file_content_hash, file_signature, generate_filename, inode, is_audio_path,
        normalize_artists, normalize_text, now, parse_artists, system_time_secs,
    },
};

struct FileSnapshot {
    meta: fs::Metadata,
    mtime: i64,
    file_signature: String,
    content_hash: String,
}

pub async fn scan_music_dir(state: AppState) -> Result<()> {
    sync_tracks(state, true).await
}

pub async fn watch_music_dir(state: AppState) {
    let (tx, mut rx) = mpsc::channel(100);
    let mut watcher = match music_watcher(tx) {
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

enum MusicWatcher {
    Recommended(RecommendedWatcher),
    Poll(PollWatcher),
}

impl MusicWatcher {
    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> notify::Result<()> {
        match self {
            Self::Recommended(watcher) => watcher.watch(path, recursive_mode),
            Self::Poll(watcher) => watcher.watch(path, recursive_mode),
        }
    }
}

fn music_watcher(tx: mpsc::Sender<notify::Result<Event>>) -> notify::Result<MusicWatcher> {
    if force_polling_watcher() {
        PollWatcher::new(
            move |result| {
                let _ = tx.blocking_send(result);
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map(MusicWatcher::Poll)
    } else {
        RecommendedWatcher::new(
            move |result| {
                let _ = tx.blocking_send(result);
            },
            Config::default(),
        )
        .map(MusicWatcher::Recommended)
    }
}

fn force_polling_watcher() -> bool {
    env::var("WATCHFILES_FORCE_POLLING")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"))
}

async fn sync_changed_paths(state: AppState, paths: Vec<PathBuf>) -> Result<()> {
    let mut current = HashSet::new();
    let mut existing = HashMap::new();
    for track in db::list_tracks(&state)? {
        existing.insert(track.file_path.clone(), track);
    }

    for path in paths {
        if path.is_dir() {
            for path in audio_paths_in_dir(&path)? {
                sync_path(&state, &existing, &mut current, &path, true).await?;
            }
        } else if path.exists() && is_audio_path(&path) {
            sync_path(&state, &existing, &mut current, &path, true).await?;
        }
    }
    for track in existing.values() {
        if !current.contains(&track.file_path) && !Path::new(&track.file_path).exists() {
            delete_missing_track(&state, track, true)?;
        }
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
            delete_missing_track(&state, track, broadcast_changes)?;
        }
    }
    Ok(())
}

fn delete_missing_track(
    state: &AppState,
    track: &crate::models::Track,
    broadcast_changes: bool,
) -> Result<()> {
    delete_track_row(state, track.id)?;
    if broadcast_changes {
        broadcast(
            state,
            "track_deleted",
            Some(&format!("Deleted track '{}'", track.title)),
            Some("info"),
            Some(json!({"id": track.id})),
        );
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
                && track.content_hash.is_some() =>
        {
            if let Some((track, file_path)) = normalize_existing_track(state, track, path).await? {
                current.insert(file_path);
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
        }
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
            let track =
                upsert_path_with_snapshot(state, path, None, None, snapshot, Some(track.id))
                    .await?;
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
                    broadcast(state, "track_added", None, None, Some(json!(track)));
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
    let _guard = state
        .mutation_lock(
            existing_id
                .map(|id| format!("track:{id}"))
                .unwrap_or_else(|| format!("path:{}", path.to_string_lossy())),
        )
        .await;
    let has_metadata_override = title_override.is_some() || artist_override.is_some();
    let (metadata, read_metadata_succeeded) = match read_metadata(path).await {
        Ok(metadata) => (metadata, true),
        Err(_) => (
            MediaMetadata {
                title: path
                    .file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or("Unknown Title")
                    .to_string(),
                artist: "Unknown Artist".into(),
                duration: 0,
                has_artist: false,
            },
            false,
        ),
    };
    let can_broadly_rename =
        has_metadata_override || (read_metadata_succeeded && metadata.has_artist);
    let title = normalize_text(&title_override.unwrap_or(metadata.title));
    let artist = normalize_artists(&artist_override.unwrap_or(metadata.artist));
    let target_path = if can_broadly_rename {
        generated_filename_path(path, &artist, &title)?
    } else {
        normalized_artist_filename(path, &artist, &title)?
    }
    .filter(|path| !path.exists());
    let update = apply_audio_update(
        path,
        &state.data_dir.join(".cache"),
        AudioUpdate {
            title: None,
            artist: None,
            cover_jpeg_path: None,
            target_path: target_path.as_deref(),
            standardize_tags: true,
            update_mtime: true,
        },
    )
    .await?;
    let path = update.path;
    let snapshot = if update.changed {
        file_snapshot(&path).await?
    } else {
        _snapshot
    };
    let file_path = path.to_string_lossy().to_string();
    let updated_at = snapshot.mtime;
    let (id, added_at) = {
        let conn = state.db.lock().unwrap();
        if let Some(id) = existing_id {
            conn.query_row(
                "UPDATE track SET title=?1, artist=?2, duration=?3, file_path=?4, updated_at=?5, inode=?6, file_signature=?7, content_hash=?8, processing_status='COMPLETE', last_error=NULL WHERE id=?9 RETURNING id, added_at",
                params![&title, &artist, metadata.duration, &file_path, updated_at, inode(&snapshot.meta), &snapshot.file_signature, &snapshot.content_hash, id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )?
        } else {
            conn.query_row(
                "INSERT INTO track (title, artist, duration, file_path, added_at, updated_at, has_cover, inode, file_signature, content_hash, processing_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0, ?6, ?7, ?8, 'COMPLETE')
                 ON CONFLICT(file_path) DO UPDATE SET title=excluded.title, artist=excluded.artist, duration=excluded.duration, updated_at=excluded.updated_at, inode=excluded.inode, file_signature=excluded.file_signature, content_hash=excluded.content_hash, processing_status='COMPLETE', last_error=NULL
                 RETURNING id, added_at",
                params![&title, &artist, metadata.duration, &file_path, updated_at, inode(&snapshot.meta), &snapshot.file_signature, &snapshot.content_hash],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )?
        }
    };
    let has_cover = extract_cover(&path, &state.covers_dir, id)
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
        updated_at,
        has_cover,
        inode: inode(&snapshot.meta),
        file_signature: Some(snapshot.file_signature),
        content_hash: Some(snapshot.content_hash),
        processing_status: "COMPLETE".into(),
        last_error: None,
    }))
}

async fn normalize_existing_track(
    state: &AppState,
    track: &crate::models::Track,
    path: &Path,
) -> Result<Option<(crate::models::TrackDto, String)>> {
    let _guard = state.mutation_lock(format!("track:{}", track.id)).await;
    let normalized_title = normalize_text(&track.title);
    let normalized_artist = normalize_artists(&track.artist);
    let target_path = normalized_artist_filename(path, &normalized_artist, &normalized_title)?;
    let target_path = if target_path.is_some() {
        target_path
    } else {
        let broad_target_path =
            generated_filename_path(path, &normalized_artist, &normalized_title)?;
        if broad_target_path.is_some()
            && matches!(read_metadata(path).await, Ok(metadata) if metadata.has_artist)
        {
            broad_target_path
        } else {
            None
        }
    }
    .filter(|path| !path.exists());

    if normalized_title == track.title && normalized_artist == track.artist && target_path.is_none()
    {
        return Ok(None);
    }

    let update = apply_audio_update(
        path,
        &state.data_dir.join(".cache"),
        AudioUpdate {
            title: (normalized_title != track.title).then_some(normalized_title.as_str()),
            artist: (normalized_artist != track.artist).then_some(normalized_artist.as_str()),
            cover_jpeg_path: None,
            target_path: target_path.as_deref(),
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await?;

    let new_path = update.path;
    let snapshot = file_snapshot(&new_path).await?;
    let mut track = track.clone();
    track.title = normalized_title;
    track.artist = normalized_artist;
    track.file_path = new_path.to_string_lossy().to_string();
    track.updated_at = snapshot.mtime;
    track.inode = inode(&snapshot.meta);
    track.file_signature = Some(snapshot.file_signature);
    track.content_hash = Some(snapshot.content_hash);
    db::save_track(state, &track)?;

    Ok(Some((
        track_dto(track),
        new_path.to_string_lossy().to_string(),
    )))
}

fn normalized_artist_filename(path: &Path, artist: &str, title: &str) -> Result<Option<PathBuf>> {
    let Some(target_path) = generated_filename_path(path, artist, title)? else {
        return Ok(None);
    };

    let Some(current_stem) = path.file_stem().and_then(|v| v.to_str()) else {
        return Ok(None);
    };
    let Some(target_stem) = target_path.file_stem().and_then(|v| v.to_str()) else {
        return Ok(None);
    };
    let Some((current_artist, current_title)) = current_stem.split_once(" - ") else {
        return Ok(None);
    };
    let Some((target_artist, target_title)) = target_stem.split_once(" - ") else {
        return Ok(None);
    };

    if normalize_text(current_title) == normalize_text(target_title)
        && parse_artists(current_artist) == parse_artists(target_artist)
    {
        return Ok(Some(target_path));
    }

    Ok(None)
}

fn generated_filename_path(path: &Path, artist: &str, title: &str) -> Result<Option<PathBuf>> {
    let Some(ext) = path.extension().and_then(|v| v.to_str()) else {
        return Ok(None);
    };
    let filename =
        generate_filename(artist, title, ext).map_err(|error| anyhow::anyhow!(error.message))?;
    let target_path = path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(filename);
    if target_path == path {
        return Ok(None);
    }

    Ok(Some(target_path))
}
