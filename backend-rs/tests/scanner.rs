use std::{fs, time::Duration};

use filetime::FileTime;
use id3::{Tag, TagLike, Version};
use mus_backend::{app::test_state, db, scanner::scan_music_dir};
use rusqlite::{params, Connection};
use tempfile::TempDir;

fn make_state() -> (TempDir, mus_backend::state::AppState) {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().to_path_buf();
    fs::create_dir_all(data_dir.join("music")).unwrap();
    fs::create_dir_all(data_dir.join("covers")).unwrap();
    let state = test_state(data_dir, Connection::open_in_memory().unwrap());
    (tmp, state)
}

fn set_mtime(path: &std::path::Path, seconds: u64) {
    filetime::set_file_mtime(
        path,
        FileTime::from_system_time(std::time::UNIX_EPOCH + Duration::from_secs(seconds)),
    )
    .unwrap();
}

#[test]
fn init_db_migrates_existing_schema() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE track (
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
        CREATE TABLE playerstate (
            id INTEGER PRIMARY KEY DEFAULT 1,
            current_track_id INTEGER,
            progress_seconds REAL NOT NULL DEFAULT 0,
            volume_level REAL NOT NULL DEFAULT 1,
            is_muted BOOLEAN NOT NULL DEFAULT 0,
            is_shuffle BOOLEAN NOT NULL DEFAULT 0,
            is_repeat BOOLEAN NOT NULL DEFAULT 0
        );
        "#,
    )
    .unwrap();

    db::init_db(&mut conn).unwrap();

    let file_signature: String = conn
        .query_row(
            "SELECT name FROM pragma_table_info('track') WHERE name = 'file_signature'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(file_signature, "file_signature");
}

#[tokio::test]
async fn scan_music_dir_finds_nested_audio_files() {
    let (_tmp, state) = make_state();
    let nested = state.music_dir.join("artist/album");
    fs::create_dir_all(&nested).unwrap();
    let path = nested.join("song.flac");
    fs::write(&path, b"not real audio").unwrap();

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].file_path, path.to_string_lossy());
}

#[tokio::test]
async fn scan_music_dir_handles_external_create_update_and_delete() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].updated_at, 2_000);

    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 4_000);
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].updated_at, 4_000);

    fs::remove_file(&path).unwrap();
    scan_music_dir(state.clone()).await.unwrap();
    assert!(db::list_tracks(&state).unwrap().is_empty());
}

#[tokio::test]
async fn scan_music_dir_skips_unchanged_existing_tracks() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1 WHERE file_path = ?2",
            params!["Kept Title", path.to_string_lossy()],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "Kept Title");
    assert!(tracks[0].file_signature.is_some());

    set_mtime(&path, 3_000);
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "Kept Title");

    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 4_000);
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].updated_at, 4_000);
}

#[tokio::test]
async fn scan_music_dir_backfills_missing_fingerprint_without_rewriting_metadata() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1, file_signature = NULL, content_hash = NULL WHERE file_path = ?2",
            params!["Kept Title", path.to_string_lossy()],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "Kept Title");
    assert!(tracks[0].file_signature.is_some());
    assert!(tracks[0].content_hash.is_some());
}

#[tokio::test]
async fn scan_music_dir_backfills_missing_hash_when_signature_exists() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1, content_hash = NULL WHERE file_path = ?2",
            params!["Kept Title", path.to_string_lossy()],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "Kept Title");
    assert!(tracks[0].file_signature.is_some());
    assert!(tracks[0].content_hash.is_some());
}

#[tokio::test]
async fn scan_music_dir_reprocesses_missing_fingerprint_with_newer_mtime() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 4_000);
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1, file_signature = NULL, content_hash = NULL WHERE file_path = ?2",
            params!["Kept Title", path.to_string_lossy()],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].updated_at, 4_000);
    assert!(tracks[0].file_signature.is_some());
    assert!(tracks[0].content_hash.is_some());
}

#[tokio::test]
async fn scan_music_dir_detects_preserved_mtime_content_changes() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1 WHERE file_path = ?2",
            params!["Kept Title", path.to_string_lossy()],
        )
        .unwrap();
    }

    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 2_000);
    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].updated_at, 2_000);
    assert!(tracks[0].content_hash.is_some());
}

#[tokio::test]
async fn scan_music_dir_standardizes_id3_tags() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.mp3");
    fs::write(&path, b"").unwrap();
    let mut tag = Tag::new();
    tag.set_title("Tagged Song");
    tag.write_to_path(&path, Version::Id3v24).unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let tag = Tag::read_from_path(&path).unwrap();
    assert_eq!(tag.version(), Version::Id3v23);
}
