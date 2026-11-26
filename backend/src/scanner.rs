use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use futures_util::{pin_mut, TryStreamExt};
use serde_json::json;
use tokio::fs as async_fs;

use crate::{
    db::{self, delete_track_row, ScanTrack},
    derived::delete_track_derived_files,
    events::broadcast,
    files::{file_content_hash, file_signature, inode, is_audio_path},
    media::{read_metadata, MediaMetadata, RejectedMedia},
    models::TrackDto,
    state::AppState,
    time::{now, system_time_secs},
};

mod paths;
mod registration;
mod watcher;
use paths::{audio_paths_in_dir, if_present, missing_path};
pub(crate) use registration::upsert_path;
use registration::{
    normalize_existing_track, relocate_registered_paths, upsert_path_with_snapshot, Relocation,
};
pub use watcher::watch_music_dir;

#[cfg(test)]
mod tests;

struct FileSnapshot {
    meta: fs::Metadata,
    mtime: i64,
    file_signature: String,
    content_hash: String,
}

struct PendingTrack<'a> {
    track: &'a ScanTrack,
    path: PathBuf,
    snapshot: FileSnapshot,
}

/// Reconciles the music directory with the database and broadcasts changes.
///
/// Scanning can normalize filenames and embedded tags. Cache files remain under
/// `.cache`, and identical copies share one library entry.
/// Exact moves keep the track ID. Ambiguous move chains are left alone while
/// unrelated files continue to sync.
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
    let _guard = state.mutation_lock("scanner".into()).await;
    let mut current = HashSet::new();
    let mut pending = Vec::new();
    let existing = index_tracks_by_path(db::scan_tracks(&state, None).await?);

    let paths = audio_paths_in_dir(&state.music_dir);
    pin_mut!(paths);
    let mut found = 0usize;
    while let Some((path, metadata)) = paths.try_next().await? {
        found += 1;
        sync_discovered_path(
            &state,
            &existing,
            &mut current,
            &mut pending,
            path,
            metadata,
        )
        .await?;
    }
    tracing::info!("found {found} audio files");
    reconcile_changed_tracks(&state, &mut current, pending).await?;

    for track in existing.values() {
        if !current.contains(&track.file_path) && !regular_audio_source(&track.file_path).await? {
            unregister_track(&state, track).await?;
        }
    }
    Ok(())
}

async fn sync_changed_paths(state: AppState, mut paths: Vec<PathBuf>) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let _guard = state.mutation_lock("scanner".into()).await;
    paths.sort();
    paths.dedup();
    let mut current = HashSet::new();
    let mut pending = Vec::new();
    let (paths, metadata) = tokio::task::spawn_blocking(move || {
        let metadata: Vec<_> = paths.iter().map(fs::metadata).collect();
        (paths, metadata)
    })
    .await?;
    // New files may have moved from anywhere. Directory changes also need
    // their descendants reconciled, so neither case can use a path-only lookup.
    let mut existing = Vec::new();
    if paths
        .iter()
        .zip(&metadata)
        .all(|(path, meta)| is_audio_path(path) && meta.as_ref().is_ok_and(fs::Metadata::is_file))
    {
        existing = db::scan_tracks(&state, Some(&paths)).await?;
    }
    if existing.len() != paths.len() {
        existing = db::scan_tracks(&state, None).await?;
    }
    let existing = index_tracks_by_path(existing);
    for (path, result) in paths.iter().zip(metadata) {
        let Some(metadata) = if_present(result)
            .with_context(|| format!("failed to inspect changed path {}", path.display()))?
        else {
            continue;
        };
        if metadata.is_dir() {
            let paths = audio_paths_in_dir(path);
            pin_mut!(paths);
            while let Some((path, metadata)) = paths.try_next().await? {
                sync_discovered_path(
                    &state,
                    &existing,
                    &mut current,
                    &mut pending,
                    path,
                    metadata,
                )
                .await?;
            }
        } else if metadata.is_file() && is_audio_path(path) {
            sync_discovered_path(
                &state,
                &existing,
                &mut current,
                &mut pending,
                path.clone(),
                metadata,
            )
            .await?;
        }
    }
    reconcile_changed_tracks(&state, &mut current, pending).await?;
    for track in existing.values() {
        if !current.contains(&track.file_path)
            && paths
                .iter()
                .any(|path| Path::new(&track.file_path).starts_with(path))
            && !regular_audio_source(&track.file_path).await?
        {
            unregister_track(&state, track).await?;
        }
    }
    Ok(())
}

fn index_tracks_by_path(tracks: Vec<ScanTrack>) -> HashMap<String, ScanTrack> {
    tracks
        .into_iter()
        .map(|track| (track.file_path.clone(), track))
        .collect()
}

async fn regular_audio_source(path: impl AsRef<Path>) -> std::io::Result<bool> {
    Ok(if_present(async_fs::metadata(path).await)?.is_some_and(|meta| meta.is_file()))
}

async fn sync_discovered_path<'a>(
    state: &AppState,
    existing: &'a HashMap<String, ScanTrack>,
    current: &mut HashSet<String>,
    pending: &mut Vec<PendingTrack<'a>>,
    path: PathBuf,
    metadata: fs::Metadata,
) -> Result<()> {
    let result = async {
        let name = path.to_string_lossy();
        if current.contains(name.as_ref()) {
            return Ok(());
        }
        let signature = file_signature(&metadata);
        if let Some(track) = existing.get(name.as_ref()) {
            if track.file_signature.as_deref() == Some(&signature) {
                if let Some((updated, file_path)) =
                    normalize_existing_track(state, track, &path).await?
                {
                    current.insert(file_path);
                    broadcast_track_update(state, updated, &track.id);
                }
                current.insert(name.into_owned());
            } else {
                let snapshot = file_snapshot_from_metadata(&path, metadata, signature).await?;
                pending.push(PendingTrack {
                    track,
                    path: path.clone(),
                    snapshot,
                });
            }
        } else {
            let Some(parsed) = read_scan_metadata(state, None, &path).await? else {
                return Ok(());
            };
            let snapshot = file_snapshot_from_metadata(&path, metadata, signature).await?;
            let mut moved = None;
            let file_inode = inode(&snapshot.meta);
            for track in existing.values() {
                if track.complete
                    && track.id == snapshot.content_hash
                    && apply_relocation(state, current, &path, None, &snapshot).await?
                {
                    return Ok(());
                }
                if moved.is_none()
                    && file_inode.is_some()
                    && track.inode == file_inode
                    && !regular_audio_source(&track.file_path).await?
                {
                    moved = Some(track);
                }
            }
            register_discovered_path(state, current, &path, snapshot, parsed, moved).await?;
        }
        Ok(())
    }
    .await;
    scan_path_result(&path, result).await
}

async fn scan_path_result(path: &Path, result: Result<()>) -> Result<()> {
    if let Err(error) = &result
        && source_disappeared(path, error).await?
    {
        tracing::debug!(path = %path.display(), "skipped vanished audio path");
        return Ok(());
    }
    result.with_context(|| format!("failed to sync audio path {}", path.display()))
}

async fn source_disappeared(path: &Path, error: &anyhow::Error) -> Result<bool> {
    let not_found = error
        .chain()
        .filter_map(|error| error.downcast_ref::<std::io::Error>())
        .any(|error| missing_path(error.kind()));
    // A missing executable also produces NotFound; verify the audio path itself.
    Ok(not_found && !regular_audio_source(path).await?)
}

async fn unregister_track(state: &AppState, track: &ScanTrack) -> Result<()> {
    let _guard = state.mutation_lock(format!("track:{}", track.id)).await;
    if db::get_track(state, &track.id)
        .await?
        .is_none_or(|registered| registered.file_path != track.file_path)
    {
        return Ok(());
    }
    delete_track_row(state, &track.id).await?;
    delete_track_derived_files(state, &track.id).await;
    broadcast_track_removed(state, track);
    Ok(())
}

fn broadcast_track_removed(state: &AppState, track: &ScanTrack) {
    broadcast(
        state,
        "track_deleted",
        Some(&format!("Removed '{}' from the library", track.title)),
        Some("info"),
        Some(json!({"id": track.id})),
    );
}

async fn reconcile_changed_tracks(
    state: &AppState,
    current: &mut HashSet<String>,
    pending: Vec<PendingTrack<'_>>,
) -> Result<()> {
    // Resolve moves before edits. Otherwise a new file at a moved track's old
    // path could take its identity before the scanner reaches its new path.
    let incoming: HashSet<_> = pending
        .iter()
        .filter(|item| item.track.id != item.snapshot.content_hash)
        .map(|item| item.snapshot.content_hash.as_str())
        .collect();
    // Start at the displaced end of an open chain. Starting in its middle
    // could delete a track whose bytes survive at another changed path.
    let is_chain_start = |item: &PendingTrack<'_>| !incoming.contains(item.track.id.as_str());
    let ordered = pending
        .iter()
        .filter(|item| is_chain_start(item))
        .chain(pending.iter().filter(|item| !is_chain_start(item)));
    for PendingTrack {
        track,
        path,
        snapshot,
    } in ordered
    {
        if current.contains(&track.file_path) || snapshot.content_hash == track.id {
            continue;
        }
        let result = apply_relocation(state, current, path, Some(track), snapshot).await;
        scan_path_result(path, result.map(|_| ())).await?;
    }
    for PendingTrack {
        track,
        path,
        snapshot,
    } in pending
    {
        if current.contains(&track.file_path) {
            continue;
        }
        let result = async {
            if track.id == snapshot.content_hash {
                if track.complete
                    || read_scan_metadata(state, Some(track), &path)
                        .await?
                        .is_some()
                {
                    current.insert(track.file_path.clone());
                    db::save_track_fingerprint(
                        state,
                        &track.id,
                        inode(&snapshot.meta),
                        &snapshot.file_signature,
                    )
                    .await?;
                }
            } else {
                register_replacement(state, current, &path, snapshot, Some(track)).await?;
            }
            Ok(())
        }
        .await;
        scan_path_result(&path, result).await?;
    }
    Ok(())
}

async fn apply_relocation(
    state: &AppState,
    current: &mut HashSet<String>,
    path: &Path,
    previous: Option<&ScanTrack>,
    snapshot: &FileSnapshot,
) -> Result<bool> {
    match relocate_registered_paths(state, path, previous, snapshot).await? {
        Relocation::NotMoved => return Ok(false),
        Relocation::Ambiguous(paths) => {
            tracing::warn!(path = %path.display(), "skipped ambiguous audio moves");
            current.extend(paths);
        }
        Relocation::Moved {
            tracks,
            displaced,
            replacement,
        } => {
            if let Some(track) = previous.filter(|_| displaced) {
                broadcast_track_removed(state, track);
            }
            for moved in tracks {
                current.insert(moved.file_path.clone());
                let id = moved.id.clone();
                broadcast_track_update(state, moved.into(), &id);
            }
            if let Some(replacement) = replacement {
                let (path, snapshot) = *replacement;
                current.insert(path.to_string_lossy().into_owned());
                let result = register_replacement(state, current, &path, snapshot, None).await;
                scan_path_result(&path, result).await?;
            }
        }
    }
    Ok(true)
}

async fn register_replacement(
    state: &AppState,
    current: &mut HashSet<String>,
    path: &Path,
    snapshot: FileSnapshot,
    previous: Option<&ScanTrack>,
) -> Result<()> {
    if let Some(parsed) = read_scan_metadata(state, previous, path).await? {
        register_discovered_path(state, current, path, snapshot, parsed, previous).await?;
    }
    Ok(())
}

async fn register_discovered_path(
    state: &AppState,
    current: &mut HashSet<String>,
    path: &Path,
    snapshot: FileSnapshot,
    metadata: MediaMetadata,
    previous: Option<&ScanTrack>,
) -> Result<()> {
    let name = path.to_string_lossy();
    let moved = previous.filter(|track| track.file_path != name);
    current.insert(name.into_owned());
    if let Some(track) = moved {
        current.insert(track.file_path.clone());
    }
    let (track, is_new, registered_path) = upsert_path_with_snapshot(
        state,
        path,
        moved.map(|track| track.title.clone()),
        moved.map(|track| track.artist.clone()),
        snapshot,
        metadata,
        previous.map(|track| track.id.clone()),
    )
    .await?;
    if let Some(previous) = previous {
        if registered_path.is_some() {
            broadcast_track_update(state, track, &previous.id);
        } else {
            unregister_track(state, previous).await?;
        }
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
    Ok(())
}

fn broadcast_track_update(state: &AppState, track: TrackDto, previous_id: &str) {
    let mut payload = json!(track);
    payload["previous_id"] = json!(previous_id);
    broadcast(state, "track_updated", None, Some("info"), Some(payload));
}

async fn read_scan_metadata(
    state: &AppState,
    track: Option<&ScanTrack>,
    path: &Path,
) -> Result<Option<MediaMetadata>> {
    match read_metadata(path).await {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.is::<RejectedMedia>() => {
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .map_or_else(|| path.to_string_lossy(), std::borrow::Cow::Borrowed);
            tracing::warn!(
                filename = %filename,
                error = %error,
                "skipping unparseable audio file"
            );
            if let Some(track) = track {
                unregister_track(state, track).await?;
            }
            Ok(None)
        }
        Err(error) => {
            Err(error).with_context(|| format!("failed to inspect audio {}", path.display()))
        }
    }
}

async fn file_snapshot(path: &Path) -> Result<FileSnapshot> {
    let meta = async_fs::metadata(path).await?;
    let signature = file_signature(&meta);
    file_snapshot_from_metadata(path, meta, signature).await
}

async fn file_snapshot_from_metadata(
    path: &Path,
    meta: fs::Metadata,
    file_signature: String,
) -> Result<FileSnapshot> {
    let mtime = meta
        .modified()
        .ok()
        .and_then(system_time_secs)
        .unwrap_or_else(now);
    let path = path.to_path_buf();
    let content_hash = tokio::task::spawn_blocking(move || file_content_hash(&path)).await??;
    Ok(FileSnapshot {
        meta,
        mtime,
        file_signature,
        content_hash,
    })
}
