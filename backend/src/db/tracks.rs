use std::path::{Path, PathBuf};

use super::{
    run,
    tags::{
        add_inferred_track_tags, load_track_tags, set_track_tags_conn, track_tags, TrackTagScope,
    },
};
use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    categories::inferred_tag_names,
    files::is_hash_id,
    media::read_cover_thumbhash,
    models::{Tag, Track},
    state::AppState,
};

/// Loads tracks in reverse discovery order, including tags in database tag order.
///
/// See the [module example](crate::db) for asynchronous library inspection.
///
/// # Errors
/// Returns worker, connection-lock, or SQLite errors.
///
/// # Panics
/// Panics if polled outside a Tokio runtime.
pub async fn list_tracks(state: &AppState) -> Result<Vec<Track>> {
    run(state, |conn| list_tracks_inner(conn, None)).await
}

pub async fn artist_values(state: &AppState) -> Result<Vec<String>> {
    run(state, |conn| {
        let mut stmt = conn.prepare("SELECT DISTINCT artist FROM track")?;
        let artists = stmt
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(artists)
    })
    .await
}

pub async fn list_tracks_by_category(state: &AppState, category: &str) -> Result<Vec<Track>> {
    let category = category.to_owned();
    run(state, move |conn| list_tracks_inner(conn, Some(&category))).await
}

fn list_tracks_inner(conn: &Connection, category: Option<&str>) -> Result<Vec<Track>> {
    let (sql, category) = if let Some(category) = category {
        (
            "SELECT track.id, title, artist, duration, file_path, added_at, updated_at,
                    has_cover, inode, file_signature, processing_status, last_error, cover_thumbhash
             FROM track
             JOIN track_tag ON track_tag.track_id = track.id
             JOIN tag ON tag.id = track_tag.tag_id
             WHERE tag.name = ?1 ORDER BY track.rowid DESC",
            Some(category),
        )
    } else {
        (
            "SELECT id, title, artist, duration, file_path, added_at, updated_at,
                    has_cover, inode, file_signature, processing_status, last_error, cover_thumbhash
             FROM track ORDER BY rowid DESC",
            None,
        )
    };
    let mut stmt = conn.prepare(sql)?;
    let mut tracks = stmt
        .query_map(rusqlite::params_from_iter(category), row_to_track)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let scope = if category.is_some() {
        TrackTagScope::Selected
    } else {
        TrackTagScope::Library
    };
    load_track_tags(conn, &mut tracks, scope)?;
    Ok(tracks)
}

pub(crate) struct ScanTrack {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub inode: Option<i64>,
    pub file_signature: Option<String>,
    pub complete: bool,
}

pub(crate) async fn scan_tracks(
    state: &AppState,
    paths: Option<&[PathBuf]>,
) -> Result<Vec<ScanTrack>> {
    let paths = paths
        .map(|paths| {
            serde_json::to_string(
                &paths
                    .iter()
                    .map(|path| path.to_string_lossy())
                    .collect::<Vec<_>>(),
            )
        })
        .transpose()?;
    run(state, move |conn| {
        let columns = "SELECT id, file_path, title, artist, inode, file_signature,
                              processing_status = 'COMPLETE' FROM track";
        let sql = if paths.is_some() {
            format!("{columns} WHERE file_path IN (SELECT value FROM json_each(?1))")
        } else {
            columns.to_owned()
        };
        let mut stmt = conn.prepare(&sql)?;
        Ok(stmt
            .query_map(rusqlite::params_from_iter(paths), |row| {
                Ok(ScanTrack {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    inode: row.get(4)?,
                    file_signature: row.get(5)?,
                    complete: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    })
    .await
}

/// Loads one track and its tags, or `None` for a missing or malformed hash ID.
///
/// # Errors
/// Returns worker, connection-lock, or SQLite errors.
///
/// # Panics
/// Panics if a database lookup is needed outside a Tokio runtime.
pub async fn get_track(state: &AppState, id: &str) -> Result<Option<Track>> {
    if !is_hash_id(id) {
        return Ok(None);
    }
    let id = id.to_owned();
    run(state, move |conn| {
        let mut track = conn
            .query_row(
                "SELECT id, title, artist, duration, file_path, added_at, updated_at,
                    has_cover, inode, file_signature, processing_status, last_error, cover_thumbhash
             FROM track WHERE id = ?1",
                params![id],
                row_to_track,
            )
            .optional()?;
        if let Some(track) = track.as_mut() {
            track.tags = track_tags(conn, &track.id)?;
        }
        Ok(track)
    })
    .await
}

pub(crate) async fn get_track_path(state: &AppState, id: &str) -> Result<Option<PathBuf>> {
    if !is_hash_id(id) {
        return Ok(None);
    }
    let id = id.to_owned();
    run(state, move |conn| {
        Ok(conn
            .prepare_cached("SELECT file_path FROM track WHERE id = ?1")?
            .query_row([id], |row| row.get::<_, String>(0))
            .optional()?
            .map(PathBuf::from))
    })
    .await
}

pub async fn save_track(
    state: &AppState,
    old_id: &str,
    track: &Track,
    tag_changes: Option<(&[String], bool)>,
) -> Result<Vec<Tag>> {
    let old_id = old_id.to_owned();
    let track = track.clone();
    let tag_changes = tag_changes.map(|(names, finalize)| (names.to_vec(), finalize));
    run(state, move |conn| {
        let tx = conn.transaction()?;
        tx.execute_batch("PRAGMA defer_foreign_keys = ON")?;
        tx.execute(
            "UPDATE track SET id=?1, title=?2, artist=?3, duration=?4, file_path=?5,
                updated_at=?6, has_cover=?7, inode=?8, file_signature=?9,
                processing_status=?10, last_error=?11, cover_thumbhash=?12 WHERE id=?13",
            params![
                track.id,
                track.title,
                track.artist,
                track.duration,
                track.file_path,
                track.updated_at,
                track.has_cover,
                track.inode,
                track.file_signature,
                track.processing_status,
                track.last_error.as_ref().map(Value::to_string),
                track.cover_thumbhash,
                old_id,
            ],
        )?;
        update_track_references(&tx, &old_id, &track.id)?;
        if let Some((tag_names, finalize_tags)) = tag_changes {
            set_track_tags_conn(&tx, &track.id, &tag_names, finalize_tags)?;
        }
        let tags = track_tags(&tx, &track.id)?;
        tx.commit()?;
        Ok(tags)
    })
    .await
}

pub(crate) async fn upsert_scanned_track(
    state: &AppState,
    track: &Track,
    existing_id: Option<&str>,
) -> Result<(i64, Vec<Tag>)> {
    let track = track.clone();
    let existing_id = existing_id.map(str::to_owned);
    run(state, move |conn| {
        let tx = conn.transaction()?;
        let added_at = if let Some(existing_id) = existing_id {
            tx.execute_batch("PRAGMA defer_foreign_keys = ON")?;
            let added_at = tx.query_row(
                "UPDATE track SET id=?1, title=?2, artist=?3, duration=?4, file_path=?5,
                    updated_at=?6, inode=?7, file_signature=?8,
                    processing_status='COMPLETE', last_error=NULL, has_cover=?9, cover_thumbhash=?10
                 WHERE id=?11 RETURNING added_at",
                params![
                    track.id,
                    track.title,
                    track.artist,
                    track.duration,
                    track.file_path,
                    track.updated_at,
                    track.inode,
                    track.file_signature,
                    track.has_cover,
                    track.cover_thumbhash,
                    existing_id,
                ],
                |row| row.get(0),
            )?;
            update_track_references(&tx, &existing_id, &track.id)?;
            added_at
        } else {
            tx.query_row(
                "INSERT INTO track (id, title, artist, duration, file_path, added_at, updated_at,
                                    has_cover, inode, file_signature, processing_status, cover_thumbhash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8, ?9, 'COMPLETE', ?10)
                 ON CONFLICT(id) DO UPDATE SET title=excluded.title, artist=excluded.artist,
                    duration=excluded.duration, file_path=excluded.file_path,
                    updated_at=excluded.updated_at, has_cover=excluded.has_cover, inode=excluded.inode,
                    file_signature=excluded.file_signature,
                    cover_thumbhash=excluded.cover_thumbhash,
                    processing_status='COMPLETE', last_error=NULL
                 RETURNING added_at",
                params![
                    track.id,
                    track.title,
                    track.artist,
                    track.duration,
                    track.file_path,
                    track.updated_at,
                    track.has_cover,
                    track.inode,
                    track.file_signature,
                    track.cover_thumbhash,
                ],
                |row| row.get(0),
            )?
        };
        add_inferred_track_tags(&tx, &track.id, &inferred_tag_names(&track.title))?;
        let tags = track_tags(&tx, &track.id)?;
        tx.commit()?;
        Ok((added_at, tags))
    })
    .await
}

fn update_track_references(conn: &Connection, old_id: &str, new_id: &str) -> Result<()> {
    if old_id != new_id {
        conn.execute(
            "UPDATE track_tag SET track_id = ?1 WHERE track_id = ?2",
            params![new_id, old_id],
        )?;
        conn.execute(
            "UPDATE playerstate SET current_track_id = ?1 WHERE current_track_id = ?2",
            params![new_id, old_id],
        )?;
    }
    Ok(())
}

pub(crate) struct TrackMove {
    pub id: String,
    pub from: String,
    pub to: String,
    pub updated_at: i64,
    pub inode: Option<i64>,
    pub file_signature: String,
}

pub(crate) async fn relocate_tracks(
    state: &AppState,
    moves: Vec<TrackMove>,
    displaced: Option<(String, String)>,
) -> Result<Vec<Track>> {
    run(state, move |conn| {
        let tx = conn.transaction()?;
        if let Some((id, path)) = displaced {
            let removed = tx.execute(
                "DELETE FROM track WHERE id = ?1 AND file_path = ?2",
                params![id, path],
            )?;
            ensure!(removed == 1, "replaced track changed during scan");
        }
        // Vacate every old path before enforcing uniqueness on the final paths.
        // These transaction-local keys cannot be filesystem paths (they contain NUL).
        let mut vacate = tx.prepare(
            "UPDATE track SET file_path = char(0) || id
             WHERE id = ?1 AND file_path = ?2 AND processing_status = 'COMPLETE'",
        )?;
        for change in &moves {
            ensure!(
                vacate.execute(params![change.id, change.from])? == 1,
                "moved track changed during scan"
            );
        }
        drop(vacate);
        let tracks = {
            let mut update = tx.prepare(
                "UPDATE track SET file_path = ?1, updated_at = ?2, inode = ?3, file_signature = ?4
                 WHERE id = ?5 RETURNING id, title, artist, duration, file_path, added_at, updated_at,
                 has_cover, inode, file_signature, processing_status, last_error, cover_thumbhash",
            )?;
            moves
                .into_iter()
                .map(|change| {
                    let mut track = update.query_row(
                        params![
                            change.to,
                            change.updated_at,
                            change.inode,
                            change.file_signature,
                            change.id
                        ],
                        row_to_track,
                    )?;
                    track.tags = track_tags(&tx, &track.id)?;
                    Ok(track)
                })
                .collect::<Result<Vec<_>>>()?
        };
        tx.commit()?;
        Ok(tracks)
    })
    .await
}

pub(crate) async fn save_track_fingerprint(
    state: &AppState,
    id: &str,
    inode: Option<i64>,
    file_signature: &str,
) -> Result<()> {
    let id = id.to_owned();
    let file_signature = file_signature.to_owned();
    run(state, move |conn| {
        conn.execute(
            "UPDATE track SET inode = ?1, file_signature = ?2 WHERE id = ?3",
            params![inode, file_signature, id],
        )?;
        Ok(())
    })
    .await
}

pub async fn delete_track_row(state: &AppState, id: &str) -> Result<()> {
    let id = id.to_owned();
    run(state, move |conn| {
        conn.execute("DELETE FROM track WHERE id = ?1", params![id])?;
        Ok(())
    })
    .await
}

/// Fills missing previews from existing cached thumbnails. Does not open or modify audio files.
///
/// Call after [`super::init_db`] on a blocking worker. Missing or invalid thumbnails are skipped.
///
/// # Errors
/// Returns SQLite query or write errors.
pub fn backfill_cover_thumbhashes(conn: &mut Connection, covers_dir: &Path) -> Result<()> {
    let ids = {
        let mut missing =
            conn.prepare("SELECT id FROM track WHERE has_cover AND cover_thumbhash IS NULL")?;
        missing
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let previews: Vec<_> = ids
        .into_iter()
        .filter(|id| is_hash_id(id))
        .filter_map(|id| {
            let hash = read_cover_thumbhash(&covers_dir.join(format!("{id}_small.webp")))?;
            Some((id, hash))
        })
        .collect();
    if previews.is_empty() {
        return Ok(());
    }

    // Decode outside the transaction. Only fill rows that still need a preview.
    let tx = conn.transaction()?;
    {
        let mut update = tx.prepare(
            "UPDATE track SET cover_thumbhash = ?1
             WHERE id = ?2 AND has_cover AND cover_thumbhash IS NULL",
        )?;
        for (id, hash) in previews {
            update.execute(params![hash, id])?;
        }
    }
    Ok(tx.commit()?)
}

pub async fn errored_tracks(state: &AppState) -> Result<Vec<Track>> {
    run(state, |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, duration, file_path, added_at, updated_at,
                    has_cover, inode, file_signature, processing_status, last_error, cover_thumbhash
             FROM track WHERE processing_status = 'ERROR'",
        )?;
        let mut tracks = stmt
            .query_map([], row_to_track)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        load_track_tags(conn, &mut tracks, TrackTagScope::Selected)?;
        Ok(tracks)
    })
    .await
}

fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    let last_error: Option<String> = row.get(11)?;
    Ok(Track {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        duration: row.get(3)?,
        file_path: row.get(4)?,
        added_at: row.get(5)?,
        updated_at: row.get(6)?,
        has_cover: row.get(7)?,
        cover_thumbhash: row.get(12)?,
        inode: row.get(8)?,
        file_signature: row.get(9)?,
        processing_status: row.get(10)?,
        last_error: last_error.and_then(|v| serde_json::from_str(&v).ok()),
        tags: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn track_and_category_edits_commit_together_and_return_canonical_tags() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&mut conn).unwrap();
        let old_id = "a".repeat(64);
        let new_id = "b".repeat(64);
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at) VALUES (?1,'Before','Artist',1,'/music/song.mp3',1)",
            [&old_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO track_tag (track_id,tag_id) SELECT ?1,id FROM tag WHERE name='ost'",
            [&old_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO playerstate (id,current_track_id) VALUES (1,?1)",
            [&old_id],
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TRIGGER reject_cover BEFORE INSERT ON track_tag
             WHEN NEW.tag_id=(SELECT id FROM tag WHERE name='cover')
             BEGIN SELECT RAISE(ABORT,'blocked category'); END;",
        )
        .unwrap();
        let directory = tempfile::TempDir::new().unwrap();
        let state = crate::test_support::test_state(directory.path().to_path_buf(), conn);
        let mut edited = get_track(&state, &old_id).await.unwrap().unwrap();
        edited.id = new_id.clone();
        edited.title = "After".into();

        let error = save_track(&state, &old_id, &edited, Some((&["cover".into()], true)))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("blocked category"));
        assert!(get_track(&state, &new_id).await.unwrap().is_none());
        let original = get_track(&state, &old_id).await.unwrap().unwrap();
        assert_eq!(original.title, "Before");
        assert_eq!(original.tags[0].name, "ost");
        assert!(!crate::db::track_tags_finalized(&state, &old_id)
            .await
            .unwrap());
        assert_eq!(
            state
                .db
                .lock()
                .unwrap()
                .query_row("SELECT current_track_id FROM playerstate", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            old_id
        );

        state
            .db
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER reject_cover")
            .unwrap();
        let tags = save_track(
            &state,
            &old_id,
            &edited,
            Some((
                &[
                    "ost".into(),
                    "cover".into(),
                    "cover".into(),
                    "unknown".into(),
                ],
                true,
            )),
        )
        .await
        .unwrap();
        assert_eq!(
            tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>(),
            ["cover", "ost"]
        );
        assert!(crate::db::track_tags_finalized(&state, &new_id)
            .await
            .unwrap());
        assert_eq!(
            state
                .db
                .lock()
                .unwrap()
                .query_row("SELECT current_track_id FROM playerstate", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            new_id
        );
    }

    #[tokio::test]
    async fn large_library_lists_tracks_and_tags_without_exhausting_sql_parameters() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&mut conn).unwrap();
        // Bundled SQLite permits 32766 SQL parameters, not one per library entry.
        conn.execute_batch(
            "WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n<32767)
             INSERT INTO track (id,title,artist,duration,file_path,added_at,processing_status)
             SELECT printf('%064x', n), 'Song', 'Artist', 1, printf('/music/%d.mp3', n), n,
                    'ERROR' FROM numbers;
             INSERT INTO track_tag (track_id,tag_id) SELECT track.id,tag.id FROM track CROSS JOIN tag WHERE tag.name IN ('ost','cover');"
        ).unwrap();

        let directory = tempfile::TempDir::new().unwrap();
        let state = crate::test_support::test_state(directory.path().to_path_buf(), conn);
        let tracks = list_tracks(&state).await.unwrap();
        assert_eq!(tracks.len(), 32767);
        assert_eq!(tracks.first().unwrap().added_at, 32767);
        assert_eq!(tracks.last().unwrap().added_at, 1);
        for track in &tracks {
            assert_eq!(
                track
                    .tags
                    .iter()
                    .map(|tag| tag.name.as_str())
                    .collect::<Vec<_>>(),
                ["cover", "ost"]
            );
        }
        let category = list_tracks_by_category(&state, "ost").await.unwrap();
        assert_eq!(category.len(), tracks.len());
        assert!(category
            .iter()
            .zip(&tracks)
            .all(|(left, right)| left.id == right.id && left.tags == right.tags));
        assert!(list_tracks_by_category(&state, "mashup")
            .await
            .unwrap()
            .is_empty());
        let failed = errored_tracks(&state).await.unwrap();
        assert_eq!(failed.len(), tracks.len());
        assert!(failed.iter().all(|track| track.tags.len() == 2));
    }
}
