use super::{
    run,
    tags::{load_track_tags, track_tags},
};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{files::is_hash_id, models::Track, state::AppState};

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
                    has_cover, inode, file_signature, content_hash, processing_status, last_error
             FROM track
             JOIN track_tag ON track_tag.track_id = track.id
             JOIN tag ON tag.id = track_tag.tag_id
             WHERE tag.name = ?1 ORDER BY track.rowid DESC",
            Some(category),
        )
    } else {
        (
            "SELECT id, title, artist, duration, file_path, added_at, updated_at,
                    has_cover, inode, file_signature, content_hash, processing_status, last_error
             FROM track ORDER BY rowid DESC",
            None,
        )
    };
    let mut stmt = conn.prepare(sql)?;
    let mut tracks = stmt
        .query_map(rusqlite::params_from_iter(category), row_to_track)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    load_track_tags(conn, &mut tracks)?;
    Ok(tracks)
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
                    has_cover, inode, file_signature, content_hash, processing_status, last_error
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

pub async fn save_track(state: &AppState, old_id: &str, track: &Track) -> Result<()> {
    let old_id = old_id.to_owned();
    let track = track.clone();
    run(state, move |conn| {
        let tx = conn.transaction()?;
        tx.execute_batch("PRAGMA defer_foreign_keys = ON")?;
        tx.execute(
            "UPDATE track SET id=?1, title=?2, artist=?3, duration=?4, file_path=?5,
                updated_at=?6, has_cover=?7, inode=?8, file_signature=?9, content_hash=?10,
                processing_status=?11, last_error=?12 WHERE id=?13",
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
                track.content_hash,
                track.processing_status,
                track.last_error.as_ref().map(Value::to_string),
                old_id,
            ],
        )?;
        update_track_references(&tx, &old_id, &track.id)?;
        tx.commit()?;
        Ok(())
    })
    .await
}

pub(crate) async fn upsert_scanned_track(
    state: &AppState,
    track: &Track,
    existing_id: Option<&str>,
) -> Result<i64> {
    let track = track.clone();
    let existing_id = existing_id.map(str::to_owned);
    run(state, move |conn| {
        if let Some(existing_id) = existing_id {
            let tx = conn.transaction()?;
            tx.execute_batch("PRAGMA defer_foreign_keys = ON")?;
            let added_at = tx.query_row(
                "UPDATE track SET id=?1, title=?2, artist=?3, duration=?4, file_path=?5,
                    updated_at=?6, inode=?7, file_signature=?8, content_hash=?9,
                    processing_status='COMPLETE', last_error=NULL
                 WHERE id=?10 RETURNING added_at",
                params![
                    track.id, track.title, track.artist, track.duration, track.file_path,
                    track.updated_at, track.inode, track.file_signature, track.content_hash,
                    existing_id,
                ],
                |row| row.get(0),
            )?;
            update_track_references(&tx, &existing_id, &track.id)?;
            tx.commit()?;
            Ok(added_at)
        } else {
            Ok(conn.query_row(
                "INSERT INTO track (id, title, artist, duration, file_path, added_at, updated_at,
                                    has_cover, inode, file_signature, content_hash, processing_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 0, ?7, ?8, ?9, 'COMPLETE')
                 ON CONFLICT(id) DO UPDATE SET title=excluded.title, artist=excluded.artist,
                    duration=excluded.duration, file_path=excluded.file_path,
                    updated_at=excluded.updated_at, inode=excluded.inode,
                    file_signature=excluded.file_signature, content_hash=excluded.content_hash,
                    processing_status='COMPLETE', last_error=NULL
                 RETURNING added_at",
                params![
                    track.id, track.title, track.artist, track.duration, track.file_path,
                    track.updated_at, track.inode, track.file_signature, track.content_hash,
                ],
                |row| row.get(0),
            )?)
        }
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

pub async fn save_track_file_signature(
    state: &AppState,
    id: &str,
    inode: Option<i64>,
    file_signature: &str,
    updated_at: i64,
) -> Result<()> {
    let id = id.to_owned();
    let file_signature = file_signature.to_owned();
    run(state, move |conn| {
        conn.execute(
            "UPDATE track SET inode = ?1, file_signature = ?2, updated_at = ?3 WHERE id = ?4",
            params![inode, file_signature, updated_at, id],
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

pub async fn set_track_cover_state(state: &AppState, id: &str, has_cover: bool) -> Result<()> {
    let id = id.to_owned();
    run(state, move |conn| {
        conn.execute(
            "UPDATE track SET has_cover = ?1 WHERE id = ?2",
            params![has_cover, id],
        )?;
        Ok(())
    })
    .await
}

pub async fn errored_tracks(state: &AppState) -> Result<Vec<Track>> {
    run(state, |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, duration, file_path, added_at, updated_at,
                    has_cover, inode, file_signature, content_hash, processing_status, last_error
             FROM track WHERE processing_status = 'ERROR'",
        )?;
        let mut tracks = stmt
            .query_map([], row_to_track)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        load_track_tags(conn, &mut tracks)?;
        Ok(tracks)
    })
    .await
}

fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    let last_error: Option<String> = row.get(12)?;
    Ok(Track {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        duration: row.get(3)?,
        file_path: row.get(4)?,
        added_at: row.get(5)?,
        updated_at: row.get(6)?,
        has_cover: row.get(7)?,
        inode: row.get(8)?,
        file_signature: row.get(9)?,
        content_hash: row.get(10)?,
        processing_status: row.get(11)?,
        last_error: last_error.and_then(|v| serde_json::from_str(&v).ok()),
        tags: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn large_library_lists_tracks_and_tags_without_exhausting_sql_parameters() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&mut conn).unwrap();
        // Bundled SQLite permits 32766 SQL parameters, not one per library entry.
        conn.execute_batch(
            "WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM numbers WHERE n<32767)
             INSERT INTO track (id,title,artist,duration,file_path,added_at,content_hash,processing_status)
             SELECT printf('%064x', n), 'Song', 'Artist', 1, printf('/music/%d.mp3', n), n,
                    printf('%064x', n), 'ERROR' FROM numbers;
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
