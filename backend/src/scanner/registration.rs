use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{ensure, Result};
use tokio::fs;

use super::{file_snapshot, file_snapshot_from_metadata, paths::if_present, FileSnapshot};
use crate::{
    audio_names::{generate_filename, normalize_artists, normalize_text, parse_artists},
    db::{self, ScanTrack, TrackMove},
    derived::delete_track_derived_files,
    files::{can_write, file_content_hash, file_signature, inode},
    media::{apply_audio_update, extract_cover, read_metadata, AudioUpdate, MediaMetadata},
    models::{Track, TrackDto},
    state::AppState,
};

// A duplicate returns no registered path, so callers do not alter the existing track.
pub(crate) async fn upsert_path(
    state: &AppState,
    path: &Path,
    title: Option<String>,
    artist: Option<String>,
) -> Result<(TrackDto, Option<PathBuf>)> {
    let metadata = read_metadata(path).await?;
    let snapshot = file_snapshot(path).await?;
    upsert_path_with_snapshot(state, path, title, artist, snapshot, metadata, None)
        .await
        .map(|(track, _, registered_path)| (track, registered_path))
}

pub(super) async fn upsert_path_with_snapshot(
    state: &AppState,
    path: &Path,
    title_override: Option<String>,
    artist_override: Option<String>,
    snapshot: FileSnapshot,
    metadata: MediaMetadata,
    existing_id: Option<String>,
) -> Result<(TrackDto, bool, Option<PathBuf>)> {
    let _guard = state
        .mutation_lock(
            existing_id
                .as_deref()
                .map(|id| format!("track:{id}"))
                .unwrap_or_else(|| format!("path:{}", path.to_string_lossy())),
        )
        .await;
    if let Some(track) = db::get_track(state, &snapshot.content_hash).await?
        && is_existing_copy(&track, &path.to_string_lossy(), existing_id.as_deref()).await?
    {
        return Ok((track.into(), false, None));
    }
    let can_broadly_rename =
        title_override.is_some() || artist_override.is_some() || metadata.has_artist;
    let title = normalize_text(&title_override.unwrap_or(metadata.title));
    let artist = normalize_artists(&artist_override.unwrap_or(metadata.artist));
    let writable = can_write(&state.music_dir).await;
    let target_path = if !writable {
        None
    } else if can_broadly_rename {
        generated_filename_path(path, &artist, &title)?
    } else {
        normalized_artist_filename(path, &artist, &title)?
    };
    let target_path = match target_path {
        Some(path) if !fs::try_exists(&path).await? => Some(path),
        _ => None,
    };
    let update = apply_audio_update(
        path,
        &state.data_dir.join(".cache"),
        AudioUpdate {
            title: None,
            artist: None,
            cover_jpeg_path: None,
            remove_section: None,
            target_path: target_path.as_deref(),
            standardize_tags: writable,
            update_mtime: true,
        },
    )
    .await?;
    let path = update.path;
    let snapshot = if update.changed {
        file_snapshot(&path).await?
    } else {
        snapshot
    };
    let file_path = path.to_string_lossy().to_string();
    let updated_at = snapshot.mtime;
    let id = snapshot.content_hash.clone();
    let registered = db::get_track(state, &id).await?;
    let is_new = existing_id.is_none() && registered.is_none();
    if let Some(track) = registered
        && is_existing_copy(&track, &file_path, existing_id.as_deref()).await?
    {
        return Ok((track.into(), false, None));
    }
    let mut track = Track {
        id: id.clone(),
        title,
        artist,
        duration: metadata.duration,
        file_path,
        added_at: updated_at,
        updated_at,
        has_cover: false,
        cover_thumbhash: None,
        inode: inode(&snapshot.meta),
        file_signature: Some(snapshot.file_signature),
        processing_status: "COMPLETE".into(),
        last_error: None,
        tags: Vec::new(),
    };
    if metadata.has_video {
        let cover = extract_cover(&path, &state.covers_dir, &id)
            .await
            .unwrap_or_default();
        track.has_cover = cover.is_some();
        track.cover_thumbhash = cover.and_then(|cover| cover.thumbhash);
    }
    (track.added_at, track.tags) =
        db::upsert_scanned_track(state, &track, existing_id.as_deref()).await?;
    if let Some(old_id) = existing_id.as_deref().filter(|old_id| *old_id != id) {
        delete_track_derived_files(state, old_id).await;
    }
    Ok((track.into(), is_new, Some(path)))
}

async fn is_existing_copy(track: &Track, path: &str, previous_id: Option<&str>) -> Result<bool> {
    if previous_id == Some(track.id.as_str())
        || track.file_path == path
        || !if_present(fs::metadata(&track.file_path).await)?.is_some_and(|meta| meta.is_file())
    {
        return Ok(false);
    }
    if previous_id.is_none() {
        return Ok(true);
    }
    // Before dropping a registered entry, confirm the other file still has these bytes.
    let original = PathBuf::from(&track.file_path);
    let hash = tokio::task::spawn_blocking(move || file_content_hash(&original)).await??;
    Ok(hash == track.id)
}

pub(super) enum Relocation {
    NotMoved,
    Ambiguous(Vec<String>),
    Moved {
        tracks: Vec<Track>,
        displaced: bool,
        replacement: Option<Box<(PathBuf, FileSnapshot)>>,
    },
}

enum MoveEnd {
    Cycle,
    Missing(String),
    Replaced(PathBuf, FileSnapshot),
}

pub(super) async fn relocate_registered_paths(
    state: &AppState,
    path: &Path,
    replaced: Option<&ScanTrack>,
    snapshot: &FileSnapshot,
) -> Result<Relocation> {
    let Some(mut source) = db::get_track(state, &snapshot.content_hash).await? else {
        return Ok(Relocation::NotMoved);
    };
    let destination = path.to_string_lossy();
    if source.file_path == destination {
        return Ok(Relocation::NotMoved);
    }
    if source.processing_status != "COMPLETE" {
        return Ok(Relocation::Ambiguous(vec![
            destination.into_owned(),
            source.file_path,
        ]));
    }

    // Follow the registered paths until the moves form a cycle or reach a
    // vacated path, which may already contain a new file.
    let mut seen = BTreeSet::new();
    let mut moves = Vec::new();
    let mut to = destination.into_owned();
    let mut next_snapshot;
    let mut at_destination = snapshot;
    let end = loop {
        if !seen.insert(source.id.clone()) {
            return Ok(ambiguous_moves(moves));
        }
        let closed = replaced.is_some_and(|track| source.id == track.id);
        let from = source.file_path.clone();
        moves.push(TrackMove {
            id: source.id,
            from: from.clone(),
            to,
            updated_at: at_destination.mtime,
            inode: inode(&at_destination.meta),
            file_signature: at_destination.file_signature.clone(),
        });
        if closed {
            ensure!(
                Path::new(&from) == path,
                "registered path changed during scan"
            );
            break MoveEnd::Cycle;
        }

        let path = Path::new(&from);
        let Some(meta) = if_present(fs::metadata(path).await)?.filter(std::fs::Metadata::is_file)
        else {
            if replaced.is_none() && moves.len() == 1 {
                return Ok(Relocation::NotMoved);
            }
            break MoveEnd::Missing(from);
        };
        let signature = file_signature(&meta);
        if source.file_signature.as_deref() == Some(&signature) {
            return Ok(copy_or_ambiguous(moves));
        }
        next_snapshot = file_snapshot_from_metadata(path, meta, signature).await?;
        if moves
            .last()
            .is_some_and(|change| change.id == next_snapshot.content_hash)
        {
            return Ok(copy_or_ambiguous(moves));
        }
        source = match db::get_track(state, &next_snapshot.content_hash).await? {
            Some(track) if track.processing_status == "COMPLETE" => track,
            Some(_) => return Ok(ambiguous_moves(moves)),
            None => break MoveEnd::Replaced(PathBuf::from(from), next_snapshot),
        };
        to = from;
        at_destination = &next_snapshot;
    };

    let displaced = replaced.filter(|_| !matches!(end, MoveEnd::Cycle));
    if let Some(track) = displaced {
        seen.insert(track.id.clone());
    }
    let mut guards = Vec::with_capacity(seen.len());
    for id in seen {
        guards.push(state.mutation_lock(format!("track:{id}")).await);
    }
    let (moves, replacement) = tokio::task::spawn_blocking(move || -> Result<_> {
        for change in &moves {
            verify_signature(Path::new(&change.to), &change.file_signature)?;
        }
        let replacement = match end {
            MoveEnd::Missing(path) => {
                ensure!(
                    !if_present(std::fs::metadata(path))?.is_some_and(|meta| meta.is_file()),
                    "audio source reappeared during scan"
                );
                None
            }
            MoveEnd::Replaced(path, snapshot) => {
                verify_signature(&path, &snapshot.file_signature)?;
                Some(Box::new((path, snapshot)))
            }
            MoveEnd::Cycle => None,
        };
        Ok((moves, replacement))
    })
    .await??;
    let removed = displaced.map(|track| (track.id.clone(), track.file_path.clone()));
    let moved = db::relocate_tracks(state, moves, removed).await?;
    if let Some(track) = displaced {
        delete_track_derived_files(state, &track.id).await;
    }
    Ok(Relocation::Moved {
        tracks: moved,
        displaced: displaced.is_some(),
        replacement,
    })
}

fn copy_or_ambiguous(moves: Vec<TrackMove>) -> Relocation {
    // A surviving copy at its own registered path needs no relocation.
    if moves.len() == 1 {
        Relocation::NotMoved
    } else {
        ambiguous_moves(moves)
    }
}

fn ambiguous_moves(moves: Vec<TrackMove>) -> Relocation {
    Relocation::Ambiguous(
        moves
            .into_iter()
            .flat_map(|change| [change.from, change.to])
            .collect(),
    )
}

fn verify_signature(path: &Path, signature: &str) -> Result<()> {
    let meta = std::fs::metadata(path)?;
    ensure!(
        meta.is_file() && file_signature(&meta) == signature,
        "audio file changed during scan"
    );
    Ok(())
}

pub(super) async fn normalize_existing_track(
    state: &AppState,
    track: &ScanTrack,
    path: &Path,
) -> Result<Option<(TrackDto, String)>> {
    // Most rows need neither a metadata edit nor equivalent filename cleanup.
    // Avoid allocating names for ASCII titles and already-trimmed single credits.
    if track.title.is_ascii()
        && track.artist.is_ascii()
        && !track.artist.contains(';')
        && track.artist.trim() == track.artist
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.split_once(" - "))
            .is_none_or(|(artist, title)| artist == track.artist && title == track.title)
    {
        return Ok(None);
    }
    let title = normalize_text(&track.title);
    let artist = normalize_artists(&track.artist);
    // Preserve saved filename choices; only normalize equivalent filename formatting.
    let target_path = normalized_artist_filename(path, &artist, &title)?;
    if title == track.title && artist == track.artist && target_path.is_none() {
        return Ok(None);
    }
    let _guard = state.mutation_lock(format!("track:{}", track.id)).await;
    // Only apply normalization to the audio snapshot we inspected.
    let Some(mut registered) = db::get_track(state, &track.id).await?.filter(|current| {
        current.file_path == track.file_path
            && current.file_signature == track.file_signature
            && current.title == track.title
            && current.artist == track.artist
    }) else {
        return Ok(None);
    };
    let target_path = match target_path {
        Some(path) if !fs::try_exists(&path).await? => Some(path),
        _ => None,
    };
    if title == track.title && artist == track.artist && target_path.is_none() {
        return Ok(None);
    }
    if !can_write(&state.music_dir).await {
        return Ok(None);
    }
    let update = apply_audio_update(
        path,
        &state.data_dir.join(".cache"),
        AudioUpdate {
            title: (title != track.title).then_some(title.as_str()),
            artist: (artist != track.artist).then_some(artist.as_str()),
            remove_section: None,
            cover_jpeg_path: None,
            target_path: target_path.as_deref(),
            standardize_tags: false,
            update_mtime: true,
        },
    )
    .await?;
    let path = update.path;
    let snapshot = file_snapshot(&path).await?;
    let old_id = track.id.clone();
    registered.id = snapshot.content_hash;
    registered.title = title;
    registered.artist = artist;
    registered.file_path = path.to_string_lossy().into_owned();
    registered.updated_at = snapshot.mtime;
    registered.inode = inode(&snapshot.meta);
    registered.file_signature = Some(snapshot.file_signature);
    if registered.has_cover && registered.id != old_id {
        let cover = extract_cover(&path, &state.covers_dir, &registered.id)
            .await
            .unwrap_or_default();
        registered.has_cover = cover.is_some();
        registered.cover_thumbhash = cover.and_then(|cover| cover.thumbhash);
    }
    registered.tags = db::save_track(state, &old_id, &registered, None).await?;
    if old_id != registered.id {
        delete_track_derived_files(state, &old_id).await;
    }
    Ok(Some((
        registered.into(),
        path.to_string_lossy().into_owned(),
    )))
}

fn normalized_artist_filename(path: &Path, artist: &str, title: &str) -> Result<Option<PathBuf>> {
    let Some((current_artist, current_title)) = path
        .file_stem()
        .and_then(|value| value.to_str())
        .and_then(|stem| stem.split_once(" - "))
    else {
        return Ok(None);
    };
    let Some(target) = generated_filename_path(path, artist, title)? else {
        return Ok(None);
    };
    let Some((target_artist, target_title)) = target
        .file_stem()
        .and_then(|value| value.to_str())
        .and_then(|stem| stem.split_once(" - "))
    else {
        return Ok(None);
    };
    Ok(
        (normalize_text(current_title) == normalize_text(target_title)
            && parse_artists(current_artist) == parse_artists(target_artist))
        .then_some(target),
    )
}

fn generated_filename_path(path: &Path, artist: &str, title: &str) -> Result<Option<PathBuf>> {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return Ok(None);
    };
    let filename = generate_filename(artist, title, ext)?;
    let target = path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(filename);
    Ok((target != path).then_some(target))
}
