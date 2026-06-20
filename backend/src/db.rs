use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    hls::track_hls_url,
    models::{PlayerState, Tag, Track, TrackDto},
    state::AppState,
    util::is_hash_id,
};

const MIGRATIONS: &[M<'static>] = &[
    M::up(
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
    ),
    M::up("ALTER TABLE track ADD COLUMN file_signature TEXT"),
    M::up(
        r#"
        DROP TABLE IF EXISTS track;
        DROP TABLE IF EXISTS playerstate;
        CREATE TABLE track (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            duration INTEGER NOT NULL,
            file_path TEXT NOT NULL UNIQUE,
            added_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            has_cover BOOLEAN NOT NULL DEFAULT 0,
            inode INTEGER,
            content_hash TEXT NOT NULL,
            processing_status TEXT NOT NULL DEFAULT 'COMPLETE',
            last_error JSON,
            file_signature TEXT
        );
        CREATE TABLE playerstate (
            id INTEGER PRIMARY KEY DEFAULT 1,
            current_track_id TEXT,
            progress_seconds REAL NOT NULL DEFAULT 0,
            volume_level REAL NOT NULL DEFAULT 1,
            is_muted BOOLEAN NOT NULL DEFAULT 0,
            is_shuffle BOOLEAN NOT NULL DEFAULT 0,
            is_repeat BOOLEAN NOT NULL DEFAULT 0
        );
        "#,
    ),
    M::up("ALTER TABLE playerstate ADD COLUMN is_playing BOOLEAN NOT NULL DEFAULT 0"),
    M::up(
        r#"
        CREATE TABLE tag (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL
        );
        CREATE TABLE track_tag (
            track_id TEXT NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (track_id, tag_id),
            FOREIGN KEY (track_id) REFERENCES track(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tag(id) ON DELETE CASCADE
        );
        CREATE INDEX idx_track_tag_tag_id ON track_tag(tag_id);
        CREATE INDEX idx_track_tag_track_id ON track_tag(track_id);
        INSERT INTO tag (name, display_name) VALUES ('gachi', 'Gachi'), ('ai-cover', 'AI cover');
        "#,
    ),
];

pub fn init_db(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
        "#,
    )?;
    Migrations::from_slice(MIGRATIONS).to_latest(conn)?;
    ensure_seed_tags(conn)?;
    Ok(())
}

fn ensure_seed_tags(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO tag (name, display_name) VALUES ('gachi', 'Gachi') ON CONFLICT(name) DO UPDATE SET display_name=excluded.display_name",
        [],
    )?;
    conn.execute(
        "INSERT INTO tag (name, display_name) VALUES ('ai-cover', 'AI cover') ON CONFLICT(name) DO UPDATE SET display_name=excluded.display_name",
        [],
    )?;
    Ok(())
}

pub fn list_tracks(state: &AppState) -> Result<Vec<Track>> {
    list_tracks_inner(state, None)
}

pub fn list_tracks_by_category(state: &AppState, category: &str) -> Result<Vec<Track>> {
    list_tracks_inner(state, Some(category))
}

fn list_tracks_inner(state: &AppState, category: Option<&str>) -> Result<Vec<Track>> {
    let conn = state.db.lock().unwrap();
    let (sql, category) = if let Some(category) = category {
        (
            "SELECT track.id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,file_signature,content_hash,processing_status,last_error FROM track JOIN track_tag ON track_tag.track_id = track.id JOIN tag ON tag.id = track_tag.tag_id WHERE tag.name = ?1 ORDER BY track.rowid DESC",
            Some(category),
        )
    } else {
        (
            "SELECT id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,file_signature,content_hash,processing_status,last_error FROM track ORDER BY rowid DESC",
            None,
        )
    };
    let mut stmt = conn.prepare(sql)?;
    let mut tracks = stmt
        .query_map(rusqlite::params_from_iter(category), row_to_track)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    load_track_tags(&conn, &mut tracks)?;
    Ok(tracks)
}

pub fn get_track(state: &AppState, id: &str) -> Result<Option<Track>> {
    if !is_hash_id(id) {
        return Ok(None);
    }
    let conn = state.db.lock().unwrap();
    let mut track = conn.query_row(
        "SELECT id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,file_signature,content_hash,processing_status,last_error FROM track WHERE id = ?1",
        params![id],
        row_to_track,
    ).optional()?;
    if let Some(track) = track.as_mut() {
        track.tags = track_tags(&conn, &track.id)?;
    }
    Ok(track)
}

pub fn save_track(state: &AppState, old_id: &str, track: &Track) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE track SET id=?1, title=?2, artist=?3, duration=?4, file_path=?5, updated_at=?6, has_cover=?7, inode=?8, file_signature=?9, content_hash=?10, processing_status=?11, last_error=?12 WHERE id=?13",
        params![track.id, track.title, track.artist, track.duration, track.file_path, track.updated_at, track.has_cover, track.inode, track.file_signature, track.content_hash, track.processing_status, track.last_error.as_ref().map(Value::to_string), old_id],
    )?;
    if old_id != track.id {
        conn.execute(
            "UPDATE track_tag SET track_id = ?1 WHERE track_id = ?2",
            params![track.id, old_id],
        )?;
        conn.execute(
            "UPDATE playerstate SET current_track_id = ?1 WHERE current_track_id = ?2",
            params![track.id, old_id],
        )?;
    }
    Ok(())
}

pub fn set_track_tags(state: &AppState, track_id: &str, tag_names: &[String]) -> Result<()> {
    let conn = state.db.lock().unwrap();
    set_track_tags_conn(&conn, track_id, tag_names)
}

pub fn add_track_tags(state: &AppState, track_id: &str, tag_names: &[String]) -> Result<()> {
    let conn = state.db.lock().unwrap();
    for name in valid_tag_names(tag_names) {
        conn.execute(
            "INSERT OR IGNORE INTO track_tag (track_id, tag_id) SELECT ?1, id FROM tag WHERE name = ?2",
            params![track_id, name],
        )?;
    }
    Ok(())
}

fn set_track_tags_conn(conn: &Connection, track_id: &str, tag_names: &[String]) -> Result<()> {
    conn.execute(
        "DELETE FROM track_tag WHERE track_id = ?1",
        params![track_id],
    )?;
    for name in valid_tag_names(tag_names) {
        conn.execute(
            "INSERT INTO track_tag (track_id, tag_id) SELECT ?1, id FROM tag WHERE name = ?2",
            params![track_id, name],
        )?;
    }
    Ok(())
}

fn valid_tag_names(tag_names: &[String]) -> Vec<&str> {
    let mut seen = HashSet::new();
    tag_names
        .iter()
        .map(String::as_str)
        .filter(|name| matches!(*name, "gachi" | "ai-cover") && seen.insert(*name))
        .collect()
}

fn load_track_tags(conn: &Connection, tracks: &mut [Track]) -> Result<()> {
    if tracks.is_empty() {
        return Ok(());
    }

    let track_ids = tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<Vec<_>>();
    let placeholders = std::iter::repeat_n("?", track_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let mut stmt = conn.prepare(&format!(
        "SELECT track_tag.track_id, tag.name, tag.display_name FROM tag JOIN track_tag ON track_tag.tag_id = tag.id WHERE track_tag.track_id IN ({placeholders}) ORDER BY track_tag.track_id, tag.id"
    ))?;
    let rows = stmt.query_map(rusqlite::params_from_iter(track_ids), |row| {
        Ok((
            row.get::<_, String>(0)?,
            Tag {
                name: row.get(1)?,
                display_name: row.get(2)?,
            },
        ))
    })?;

    let mut tags_by_track: HashMap<String, Vec<Tag>> = HashMap::new();
    for row in rows {
        let (track_id, tag) = row?;
        tags_by_track.entry(track_id).or_default().push(tag);
    }
    for track in tracks {
        track.tags = tags_by_track.remove(&track.id).unwrap_or_default();
    }
    Ok(())
}

fn track_tags(conn: &Connection, track_id: &str) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT tag.name, tag.display_name FROM tag JOIN track_tag ON track_tag.tag_id = tag.id WHERE track_tag.track_id = ?1 ORDER BY tag.id",
    )?;
    let tags = stmt
        .query_map(params![track_id], |row| {
            Ok(Tag {
                name: row.get(0)?,
                display_name: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}

pub fn save_track_fingerprint(
    state: &AppState,
    id: &str,
    inode: Option<i64>,
    file_signature: &str,
    content_hash: &str,
) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "UPDATE track SET id = ?3, inode = ?1, file_signature = ?2, content_hash = ?3 WHERE id = ?4",
        params![inode, file_signature, content_hash, id],
    )?;
    if id != content_hash {
        conn.execute(
            "UPDATE playerstate SET current_track_id = ?1 WHERE current_track_id = ?2",
            params![content_hash, id],
        )?;
    }
    Ok(())
}

pub fn delete_track_row(state: &AppState, id: &str) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute("DELETE FROM track WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn set_track_cover_state(state: &AppState, id: &str, has_cover: bool) -> Result<()> {
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
            "SELECT current_track_id, progress_seconds, volume_level, is_muted, is_playing, is_shuffle, is_repeat FROM playerstate WHERE id = 1",
            [],
            |row| Ok(PlayerState {
                current_track_id: row.get(0)?,
                progress_seconds: row.get(1)?,
                volume_level: row.get(2)?,
                is_muted: row.get(3)?,
                is_playing: row.get(4)?,
                is_shuffle: row.get(5)?,
                is_repeat: row.get(6)?,
            }),
        )
        .optional()?
        .unwrap_or(PlayerState {
            current_track_id: None,
            progress_seconds: 0.0,
            volume_level: 1.0,
            is_muted: false,
            is_playing: false,
            is_shuffle: false,
            is_repeat: false,
        }))
}

pub fn save_player_state(state: &AppState, player_state: &PlayerState) -> Result<()> {
    let conn = state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO playerstate (id, current_track_id, progress_seconds, volume_level, is_muted, is_playing, is_shuffle, is_repeat)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
         current_track_id=excluded.current_track_id,
         progress_seconds=excluded.progress_seconds,
         volume_level=excluded.volume_level,
         is_muted=excluded.is_muted,
         is_playing=excluded.is_playing,
         is_shuffle=excluded.is_shuffle,
         is_repeat=excluded.is_repeat",
        params![
            player_state.current_track_id,
            player_state.progress_seconds,
            player_state.volume_level,
            player_state.is_muted,
            player_state.is_playing,
            player_state.is_shuffle,
            player_state.is_repeat
        ],
    )?;
    Ok(())
}

pub fn errored_tracks(state: &AppState) -> Result<Vec<Track>> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,file_signature,content_hash,processing_status,last_error FROM track WHERE processing_status = 'ERROR'")?;
    let mut tracks = stmt
        .query_map([], row_to_track)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    load_track_tags(&conn, &mut tracks)?;
    Ok(tracks)
}

pub fn track_dto(track: Track) -> TrackDto {
    let cover_version = track.content_hash.clone();
    let cover_small_url = track.has_cover.then(|| {
        format!(
            "/api/v1/tracks/{}/covers/small.webp?v={}",
            track.id, cover_version
        )
    });
    let cover_original_url = track.has_cover.then(|| {
        format!(
            "/api/v1/tracks/{}/covers/original.webp?v={}",
            track.id, cover_version
        )
    });
    TrackDto {
        hls_url: track_hls_url(&track),
        id: track.id.clone(),
        title: track.title,
        artist: track.artist,
        duration: track.duration,
        filename: Path::new(&track.file_path)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_string(),
        updated_at: track.updated_at,
        has_cover: track.has_cover,
        cover_small_url,
        cover_original_url,
        tags: track.tags,
    }
}

pub fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
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
