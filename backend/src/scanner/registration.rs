use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::fs;

use super::{file_snapshot, FileSnapshot};
use crate::{
    audio_names::{generate_filename, normalize_artists, normalize_text, parse_artists},
    categories::inferred_tag_names,
    db,
    derived::delete_track_derived_files,
    files::inode,
    hls::prewarm_track_hls,
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
    if existing_id.is_none()
        && let Some(track) = db::get_track(state, &snapshot.content_hash).await?
        && track.file_path != path.to_string_lossy()
        && fs::metadata(&track.file_path)
            .await
            .is_ok_and(|meta| meta.is_file())
    {
        return Ok((track.into(), false, None));
    }
    let can_broadly_rename =
        title_override.is_some() || artist_override.is_some() || metadata.has_artist;
    let title = normalize_text(&title_override.unwrap_or(metadata.title));
    let artist = normalize_artists(&artist_override.unwrap_or(metadata.artist));
    let target_path = if can_broadly_rename {
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
            standardize_tags: true,
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
    if existing_id.is_none()
        && let Some(track) = registered
        && track.file_path != file_path
        && fs::metadata(&track.file_path)
            .await
            .is_ok_and(|meta| meta.is_file())
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
        inode: inode(&snapshot.meta),
        file_signature: Some(snapshot.file_signature),
        content_hash: snapshot.content_hash,
        processing_status: "COMPLETE".into(),
        last_error: None,
        tags: Vec::new(),
    };
    track.added_at = db::upsert_scanned_track(state, &track, existing_id.as_deref()).await?;
    db::add_inferred_track_tags(state, &id, &inferred_tag_names(&track.title)).await?;
    track.has_cover = extract_cover(&path, &state.covers_dir, &id)
        .await
        .unwrap_or(false);
    db::set_track_cover_state(state, &id, track.has_cover).await?;
    track.tags = db::get_track(state, &id)
        .await?
        .map(|track| track.tags)
        .unwrap_or_default();
    prewarm_track_hls(state, &track).await;
    Ok((track.into(), is_new, Some(path)))
}

pub(super) async fn normalize_existing_track(
    state: &AppState,
    track: &Track,
    path: &Path,
) -> Result<Option<(TrackDto, String)>> {
    let _guard = state.mutation_lock(format!("track:{}", track.id)).await;
    let title = normalize_text(&track.title);
    let artist = normalize_artists(&track.artist);
    let target_path = match normalized_artist_filename(path, &artist, &title)? {
        Some(path) => Some(path),
        None => {
            let target = generated_filename_path(path, &artist, &title)?;
            if target.is_some()
                && matches!(read_metadata(path).await, Ok(metadata) if metadata.has_artist)
            {
                target
            } else {
                None
            }
        }
    };
    let target_path = match target_path {
        Some(path) if !fs::try_exists(&path).await? => Some(path),
        _ => None,
    };
    if title == track.title && artist == track.artist && target_path.is_none() {
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
    let mut track = track.clone();
    track.id = snapshot.content_hash.clone();
    track.title = title;
    track.artist = artist;
    track.file_path = path.to_string_lossy().to_string();
    track.updated_at = snapshot.mtime;
    track.inode = inode(&snapshot.meta);
    track.file_signature = Some(snapshot.file_signature);
    track.content_hash = snapshot.content_hash;
    if track.has_cover {
        track.has_cover = extract_cover(&path, &state.covers_dir, &track.id)
            .await
            .unwrap_or(false);
    }
    db::save_track(state, &old_id, &track).await?;
    if old_id != track.id {
        delete_track_derived_files(state, &old_id).await;
    }
    prewarm_track_hls(state, &track).await;
    Ok(Some((track.into(), path.to_string_lossy().to_string())))
}

fn normalized_artist_filename(path: &Path, artist: &str, title: &str) -> Result<Option<PathBuf>> {
    let Some(target) = generated_filename_path(path, artist, title)? else {
        return Ok(None);
    };
    let Some(current_stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(None);
    };
    let Some(target_stem) = target.file_stem().and_then(|value| value.to_str()) else {
        return Ok(None);
    };
    let Some((current_artist, current_title)) = current_stem.split_once(" - ") else {
        return Ok(None);
    };
    let Some((target_artist, target_title)) = target_stem.split_once(" - ") else {
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
