use std::{
    collections::HashMap,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use futures_util::{pin_mut, TryStreamExt};
use serde_json::json;
use tokio::fs as async_fs;

use crate::{
    db::{self, delete_track_row},
    derived::delete_track_derived_files,
    events::broadcast,
    files::{file_content_hash, file_signature, inode, is_audio_path},
    hls::prewarm_track_hls,
    media::{read_metadata, MediaMetadata, RejectedMedia},
    models::TrackDto,
    state::AppState,
    time::{now, system_time_secs},
};

mod paths;
mod registration;
mod watcher;
use paths::audio_paths_in_dir;
pub(crate) use registration::upsert_path;
use registration::{normalize_existing_track, upsert_path_with_snapshot};
pub use watcher::watch_music_dir;

#[cfg(test)]
mod tests;

struct FileSnapshot {
    meta: fs::Metadata,
    mtime: i64,
    file_signature: String,
    content_hash: String,
}

/// Reconciles the music directory with the database and broadcasts changes.
///
/// Scanning can normalize filenames and embedded tags. Cache files remain under
/// `.cache`, and identical copies share one library entry.
///
/// # Errors
/// Returns directory, media, database, or worker failures with operation context.
/// Files for which ffprobe exits with code 1 are skipped and removed from the
/// library, not deleted. Launch, timeout, malformed-output, and abnormal-exit
/// failures abort scanning without removing the affected track's record.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
///
/// ```no_run
/// # async fn example(state: mus_backend::state::AppState) -> anyhow::Result<()> {
/// mus_backend::scanner::scan_music_dir(state).await?;
/// # Ok(())
/// # }
/// ```
pub async fn scan_music_dir(state: AppState) -> Result<()> {
    sync_tracks(state, true).await
}

async fn sync_changed_paths(state: AppState, paths: Vec<PathBuf>) -> Result<()> {
    let _guard = state.mutation_lock("scanner".into()).await;
    let mut current = HashSet::new();
    let mut existing = HashMap::new();
    for track in db::list_tracks(&state).await? {
        existing.insert(track.file_path.clone(), track);
    }

    let mut paths = paths;
    paths.sort();
    paths.dedup();
    for path in &paths {
        let metadata = match async_fs::metadata(path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to inspect changed path {}", path.display()));
            }
        };
        if metadata.is_dir() {
            let paths = audio_paths_in_dir(path);
            pin_mut!(paths);
            while let Some(path) = paths.try_next().await? {
                match sync_path(&state, &existing, &mut current, &path, true).await {
                    Ok(()) => {}
                    Err(error) if source_disappeared(&path, &error).await? => {
                        tracing::debug!(path = %path.display(), "skipped vanished changed path");
                    }
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("failed to sync changed path {}", path.display())
                        });
                    }
                }
            }
        } else if is_audio_path(path) {
            match sync_path(&state, &existing, &mut current, path, true).await {
                Ok(()) => {}
                Err(error) if source_disappeared(path, &error).await? => {
                    tracing::debug!(path = %path.display(), "skipped vanished changed path");
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("failed to sync changed path {}", path.display())
                    });
                }
            }
        }
    }
    for track in existing.values() {
        if paths
            .iter()
            .any(|path| Path::new(&track.file_path).starts_with(path))
            && !current.contains(&track.file_path)
            && !async_fs::try_exists(&track.file_path).await?
        {
            delete_missing_track(&state, track, true).await?;
        }
    }
    Ok(())
}

async fn sync_tracks(state: AppState, broadcast_changes: bool) -> Result<()> {
    let _guard = state.mutation_lock("scanner".into()).await;
    let mut current = HashSet::new();
    let mut existing = HashMap::new();
    for track in db::list_tracks(&state).await? {
        existing.insert(track.file_path.clone(), track);
    }

    let paths = audio_paths_in_dir(&state.music_dir);
    pin_mut!(paths);
    let mut found = 0usize;
    while let Some(path) = paths.try_next().await? {
        found += 1;
        match sync_path(&state, &existing, &mut current, &path, broadcast_changes).await {
            Ok(()) => {}
            Err(error) if source_disappeared(&path, &error).await? => {
                tracing::debug!(path = %path.display(), "skipped vanished scanned path");
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to sync scanned path {}", path.display()));
            }
        }
    }
    tracing::info!("found {found} audio files");

    for track in existing.values() {
        if !current.contains(&track.file_path) && !async_fs::try_exists(&track.file_path).await? {
            delete_missing_track(&state, track, broadcast_changes).await?;
        }
    }
    Ok(())
}

async fn source_disappeared(path: &Path, error: &anyhow::Error) -> Result<bool> {
    let not_found = error
        .chain()
        .filter_map(|error| error.downcast_ref::<std::io::Error>())
        .any(|error| error.kind() == std::io::ErrorKind::NotFound);
    // A missing executable also produces NotFound; verify the audio path itself.
    Ok(not_found && !async_fs::try_exists(path).await?)
}

async fn delete_missing_track(
    state: &AppState,
    track: &crate::models::Track,
    broadcast_changes: bool,
) -> Result<()> {
    if db::get_track(state, &track.id)
        .await?
        .is_some_and(|registered| registered.file_path != track.file_path)
    {
        return Ok(());
    }
    delete_track_row(state, &track.id).await?;
    delete_track_derived_files(state, &track.id).await;
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
    let (meta, mtime, signature) = file_metadata(path).await?;
    let Some(metadata) =
        read_scan_metadata(state, existing, &path_string, path, broadcast_changes).await?
    else {
        return Ok(());
    };
    current.insert(path_string.clone());
    match existing.get(&path_string) {
        Some(track) if track.file_signature.as_deref() == Some(signature.as_str()) => {
            if let Some((updated, file_path)) = normalize_existing_track(state, track, path).await?
            {
                current.insert(file_path);
                if broadcast_changes {
                    broadcast_track_update(state, updated, &track.id);
                }
            } else {
                prewarm_track_hls(state, track).await;
            }
        }
        Some(track) => {
            let snapshot = file_snapshot_from_metadata(path, meta, mtime, signature).await?;
            if track.content_hash == snapshot.content_hash {
                db::save_track_fingerprint(
                    state,
                    &track.id,
                    inode(&snapshot.meta),
                    &snapshot.file_signature,
                )
                .await?;
                prewarm_track_hls(state, track).await;
                return Ok(());
            }
            let (updated, _, _) = upsert_path_with_snapshot(
                state,
                path,
                None,
                None,
                snapshot,
                metadata,
                Some(track.id.clone()),
            )
            .await?;
            if broadcast_changes {
                broadcast_track_update(state, updated, &track.id);
            }
        }
        None => {
            let snapshot = file_snapshot_from_metadata(path, meta, mtime, signature).await?;
            let mut moved_track = None;
            if snapshot.meta.is_file() {
                for track in existing.values() {
                    if track.inode == inode(&snapshot.meta)
                        && track.inode.is_some()
                        && !async_fs::try_exists(&track.file_path).await?
                    {
                        moved_track = Some(track);
                        break;
                    }
                }
            }
            if let Some(track) = moved_track {
                current.insert(track.file_path.clone());
            }
            let (track, is_new, _) = upsert_path_with_snapshot(
                state,
                path,
                moved_track.map(|track| track.title.clone()),
                moved_track.map(|track| track.artist.clone()),
                snapshot,
                metadata,
                moved_track.map(|track| track.id.clone()),
            )
            .await?;
            if broadcast_changes {
                if let Some(previous) = moved_track {
                    broadcast_track_update(state, track, &previous.id);
                } else if is_new {
                    let filename = track.filename.clone();
                    broadcast(
                        state,
                        "track_added",
                        Some(&filename),
                        Some("info"),
                        Some(json!(track)),
                    );
                }
            }
        }
    }
    Ok(())
}

fn broadcast_track_update(state: &AppState, track: TrackDto, previous_id: &str) {
    let mut payload = json!(track);
    payload["previous_id"] = json!(previous_id);
    broadcast(state, "track_updated", None, Some("info"), Some(payload));
}

async fn read_scan_metadata(
    state: &AppState,
    existing: &HashMap<String, crate::models::Track>,
    path_string: &str,
    path: &Path,
    broadcast_changes: bool,
) -> Result<Option<MediaMetadata>> {
    match read_metadata(path).await {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.is::<RejectedMedia>() => {
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(path_string);
            tracing::warn!(
                filename,
                error = %error,
                "skipping unparseable audio file"
            );
            if let Some(track) = existing.get(path_string) {
                delete_missing_track(state, track, broadcast_changes).await?;
            }
            Ok(None)
        }
        Err(error) => {
            Err(error).with_context(|| format!("failed to inspect audio {}", path.display()))
        }
    }
}

async fn file_metadata(path: &Path) -> Result<(fs::Metadata, i64, String)> {
    let meta = async_fs::metadata(path).await?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(system_time_secs)
        .unwrap_or_else(now);
    let signature = file_signature(&meta);
    Ok((meta, mtime, signature))
}

async fn file_snapshot(path: &Path) -> Result<FileSnapshot> {
    let (meta, mtime, signature) = file_metadata(path).await?;
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
