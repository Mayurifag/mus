use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::{
    models::{PlayerState, Track, TrackDto},
    state::AppState,
};

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS track (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            duration INTEGER NOT NULL,
            file_path TEXT NOT NULL UNIQUE,
            added_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            has_cover BOOLEAN NOT NULL DEFAULT 0,
            inode INTEGER,
            content_hash TEXT,
            processing_status TEXT NOT NULL DEFAULT 'COMPLETE',
            last_error JSON
        );
        CREATE INDEX IF NOT EXISTS ix_track_file_path ON track(file_path);
        CREATE INDEX IF NOT EXISTS ix_track_inode ON track(inode);
        CREATE TABLE IF NOT EXISTS playerstate (
            id INTEGER PRIMARY KEY DEFAULT 1,
            current_track_id INTEGER,
            progress_seconds REAL NOT NULL DEFAULT 0,
            volume_level REAL NOT NULL DEFAULT 1,
            is_muted BOOLEAN NOT NULL DEFAULT 0,
            is_shuffle BOOLEAN NOT NULL DEFAULT 0,
            is_repeat BOOLEAN NOT NULL DEFAULT 0
        );
        "#,
    )?;
    Ok(())
}

pub fn list_tracks(state: &AppState) -> Result<Vec<Track>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,content_hash,processing_status,last_error FROM track ORDER BY added_at DESC")?;
    let tracks = stmt
        .query_map([], row_to_track)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tracks)
}

pub fn get_track(state: &AppState, id: i64) -> Result<Option<Track>> {
    let conn = state.db.lock().unwrap();
    conn.query_row(
        "SELECT id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,content_hash,processing_status,last_error FROM track WHERE id = ?1",
        params![id],
        row_to_track,
    ).optional().map_err(Into::into)
}

pub fn save_track(state: &AppState, track: &Track) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE track SET title=?1, artist=?2, duration=?3, file_path=?4, updated_at=?5, has_cover=?6, processing_status=?7, last_error=?8 WHERE id=?9",
        params![track.title, track.artist, track.duration, track.file_path, track.updated_at, track.has_cover, track.processing_status, track.last_error.as_ref().map(Value::to_string), track.id],
    )?;
    Ok(())
}

pub fn delete_track_row(state: &AppState, id: i64) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM track WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn set_track_cover_state(state: &AppState, id: i64, has_cover: bool) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE track SET has_cover = ?1 WHERE id = ?2",
        params![has_cover, id],
    )?;
    Ok(())
}

pub fn load_player_state(state: &AppState) -> Result<PlayerState> {
    let conn = state.db.lock().unwrap();
    Ok(conn
        .query_row(
            "SELECT current_track_id, progress_seconds, volume_level, is_muted, is_shuffle, is_repeat FROM playerstate WHERE id = 1",
            [],
            |row| Ok(PlayerState {
                current_track_id: row.get(0)?,
                progress_seconds: row.get(1)?,
                volume_level: row.get(2)?,
                is_muted: row.get(3)?,
                is_shuffle: row.get(4)?,
                is_repeat: row.get(5)?,
            }),
        )
        .optional()?
        .unwrap_or(PlayerState {
            current_track_id: None,
            progress_seconds: 0.0,
            volume_level: 1.0,
            is_muted: false,
            is_shuffle: false,
            is_repeat: false,
        }))
}

pub fn save_player_state(state: &AppState, player_state: &PlayerState) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO playerstate (id, current_track_id, progress_seconds, volume_level, is_muted, is_shuffle, is_repeat)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
         current_track_id=excluded.current_track_id,
         progress_seconds=excluded.progress_seconds,
         volume_level=excluded.volume_level,
         is_muted=excluded.is_muted,
         is_shuffle=excluded.is_shuffle,
         is_repeat=excluded.is_repeat",
        params![
            player_state.current_track_id,
            player_state.progress_seconds,
            player_state.volume_level,
            player_state.is_muted,
            player_state.is_shuffle,
            player_state.is_repeat
        ],
    )?;
    Ok(())
}

pub fn errored_tracks(state: &AppState) -> Result<Vec<Track>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,content_hash,processing_status,last_error FROM track WHERE processing_status = 'ERROR'")?;
    let tracks = stmt
        .query_map([], row_to_track)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tracks)
}

pub fn track_dto(track: Track) -> TrackDto {
    let cover_small_url = track.has_cover.then(|| {
        format!(
            "/api/v1/tracks/{}/covers/small.webp?v={}",
            track.id, track.updated_at
        )
    });
    let cover_original_url = track.has_cover.then(|| {
        format!(
            "/api/v1/tracks/{}/covers/original.webp?v={}",
            track.id, track.updated_at
        )
    });
    TrackDto {
        id: track.id,
        title: track.title,
        artist: track.artist,
        duration: track.duration,
        file_path: track.file_path,
        updated_at: track.updated_at,
        has_cover: track.has_cover,
        cover_small_url,
        cover_original_url,
    }
}

pub fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
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
        inode: row.get(8)?,
        content_hash: row.get(9)?,
        processing_status: row.get(10)?,
        last_error: last_error.and_then(|v| serde_json::from_str(&v).ok()),
    })
}
