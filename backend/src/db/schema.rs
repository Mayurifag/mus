use anyhow::Result;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use super::tags::{ensure_inferred_category_tags, ensure_seed_tags, remove_deprecated_tags};

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
        INSERT INTO tag (name, display_name) VALUES ('right-version', 'Right version'), ('ai-cover', 'AI cover'), ('cover', 'Cover'), ('ost', 'OST'), ('mashup', 'Mashup');
        "#,
    ),
    M::up("ALTER TABLE track ADD COLUMN tags_finalized BOOLEAN NOT NULL DEFAULT 0"),
];

/// Enables SQLite settings, applies migrations, seeds tags, and infers pending categories.
///
/// Performs blocking database work; async startup must call it on a blocking worker.
///
/// # Errors
/// Returns SQLite or migration errors without discarding their causes.
///
/// ```
/// let mut connection = rusqlite::Connection::open_in_memory()?;
/// mus_backend::db::init_db(&mut connection)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn init_db(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
        "#,
    )?;
    Migrations::from_slice(MIGRATIONS).to_latest(conn)?;
    ensure_seed_tags(conn)?;
    remove_deprecated_tags(conn)?;
    ensure_inferred_category_tags(conn)?;
    Ok(())
}
