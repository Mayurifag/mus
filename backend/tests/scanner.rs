use std::{fs, path::Path, process::Command, time::Duration};

use filetime::FileTime;
use id3::{Tag, TagLike, Version};
use mus_backend::{
    app::test_state,
    db,
    scanner::scan_music_dir,
    util::{file_content_hash, file_signature, inode},
};
use rusqlite::{params, Connection};
use tempfile::TempDir;
use tokio::time::timeout;

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

fn create_mp3(path: &Path, title: &str, artist: &str) {
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.1",
            "-c:a",
            "libmp3lame",
            "-q:a",
            "9",
            "-metadata",
            &format!("title={title}"),
            "-metadata",
            &format!("artist={artist}"),
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_silent_wav(path: &Path) {
    let sample_rate = 8_000u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let samples = sample_rate / 10;
    let data_len = samples * channels as u32 * bits_per_sample as u32 / 8;
    let mut data = Vec::new();
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + data_len).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&channels.to_le_bytes());
    data.extend_from_slice(&sample_rate.to_le_bytes());
    data.extend_from_slice(
        &(sample_rate * channels as u32 * bits_per_sample as u32 / 8).to_le_bytes(),
    );
    data.extend_from_slice(&(channels * bits_per_sample / 8).to_le_bytes());
    data.extend_from_slice(&bits_per_sample.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&data_len.to_le_bytes());
    data.resize(data.len() + data_len as usize, 0);
    fs::write(path, data).unwrap();
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
    let path = nested.join("song.wav");
    write_silent_wav(&path);

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].file_path, path.to_string_lossy());
}

#[tokio::test]
async fn scan_music_dir_skips_unparseable_audio_files() {
    let (_tmp, state) = make_state();
    fs::write(state.music_dir.join("broken.mp3"), b"not real audio").unwrap();

    scan_music_dir(state.clone()).await.unwrap();

    assert!(db::list_tracks(&state).unwrap().is_empty());
}

#[tokio::test]
async fn scan_music_dir_removes_existing_unparseable_audio_files() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("broken.mp3");
    fs::write(&path, b"not real audio").unwrap();
    let id = "1111111111111111111111111111111111111111111111111111111111111111";
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,content_hash,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,?1,'COMPLETE')",
            params![id, "broken", "Artist", 0, path.to_string_lossy(), 100, 100],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    assert!(db::list_tracks(&state).unwrap().is_empty());
}

#[tokio::test]
async fn list_tracks_keeps_discovery_order_when_updated_at_changes() {
    let (_tmp, state) = make_state();
    let first = state.music_dir.join("first.flac");
    let second = state.music_dir.join("second.flac");
    fs::write(&first, b"first").unwrap();
    fs::write(&second, b"second").unwrap();
    let first_id = "1111111111111111111111111111111111111111111111111111111111111111";
    let second_id = "2222222222222222222222222222222222222222222222222222222222222222";
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,content_hash,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,?1,'COMPLETE')",
            params![first_id, "First", "Artist", 1, first.to_string_lossy(), 100, 100],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,content_hash,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,?1,'COMPLETE')",
            params![second_id, "Second", "Artist", 1, second.to_string_lossy(), 200, 200],
        )
        .unwrap();
    }

    assert_eq!(
        db::list_tracks(&state)
            .unwrap()
            .into_iter()
            .map(|track| track.id)
            .collect::<Vec<_>>(),
        vec![second_id, first_id]
    );

    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET updated_at = 9999 WHERE id = ?1",
            params![first_id],
        )
        .unwrap();
    }

    assert_eq!(
        db::list_tracks(&state)
            .unwrap()
            .into_iter()
            .map(|track| track.id)
            .collect::<Vec<_>>(),
        vec![second_id, first_id]
    );
}

#[tokio::test]
async fn scan_music_dir_prewarms_existing_unchanged_tracks() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("Artist - song.mp3");
    create_mp3(&path, "song", "Artist");
    let meta = fs::metadata(&path).unwrap();
    let content_hash = file_content_hash(&path).unwrap();
    let signature = file_signature(&meta);
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,file_signature,content_hash,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,?8,?9,?10,'COMPLETE')",
            params![&content_hash, "song", "Artist", 1, path.to_string_lossy(), 100, 100, inode(&meta), signature, &content_hash],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    let hls_dir = state
        .data_dir
        .join(".cache/hls")
        .join(&content_hash)
        .join(&content_hash);
    assert!(hls_dir.join("index.m3u8").is_file());
    assert!(fs::read_to_string(hls_dir.join("index.m3u8"))
        .unwrap()
        .contains("#EXT-X-ENDLIST"));
}

#[tokio::test]
async fn scan_music_dir_broadcasts_added_tracks() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.wav");
    write_silent_wav(&path);
    let mut events = state.events.subscribe();

    scan_music_dir(state.clone()).await.unwrap();

    let event = timeout(Duration::from_secs(1), events.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_added"));
    assert_eq!(event.message_to_show.as_deref(), Some("song.wav"));
    assert_eq!(event.message_level.as_deref(), Some("info"));
    assert_eq!(event.action_payload.unwrap()["title"], "song");
}

#[tokio::test]
async fn scan_music_dir_normalizes_comma_artists_and_renames_filename() {
    let (_tmp, state) = make_state();
    let path = state
        .music_dir
        .join("aikko,katanacss,INSPACE,playingtheangel - song.mp3");
    create_mp3(&path, "song", "aikko,katanacss,INSPACE,playingtheangel");

    scan_music_dir(state.clone()).await.unwrap();

    let renamed_path = state
        .music_dir
        .join("aikko, katanacss, INSPACE, playingtheangel - song.mp3");
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(
        tracks[0].artist,
        "aikko; katanacss; INSPACE; playingtheangel"
    );
    assert_eq!(tracks[0].file_path, renamed_path.to_string_lossy());
    assert!(!path.exists());
    assert!(renamed_path.exists());
}

#[tokio::test]
async fn scan_music_dir_renames_unstructured_filename_from_metadata() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("random-name.mp3");
    create_mp3(&path, "song", "Artist");

    scan_music_dir(state.clone()).await.unwrap();

    let renamed_path = state.music_dir.join("Artist - song.mp3");
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].artist, "Artist");
    assert_eq!(tracks[0].file_path, renamed_path.to_string_lossy());
    let hls_dir = state
        .data_dir
        .join(".cache/hls")
        .join(&tracks[0].id)
        .join(&tracks[0].content_hash);
    assert!(hls_dir.join("index.m3u8").is_file());
    assert!(fs::read_to_string(hls_dir.join("index.m3u8"))
        .unwrap()
        .contains("#EXT-X-ENDLIST"));
    assert!(!path.exists());
    assert!(renamed_path.exists());
}

#[tokio::test]
async fn scan_music_dir_normalizes_unicode_filenames_to_nfc() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("random-name.mp3");
    create_mp3(&path, "Бедныи\u{0306} Русскии\u{0306}", "Слава КПСС");

    scan_music_dir(state.clone()).await.unwrap();

    let renamed_path = state.music_dir.join("Слава КПСС - Бедный Русский.mp3");
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "Бедный Русский");
    assert_eq!(tracks[0].file_path, renamed_path.to_string_lossy());
    assert!(!path.exists());
    assert!(renamed_path.exists());
}

#[tokio::test]
async fn scan_music_dir_normalizes_unchanged_existing_tracks() {
    let (_tmp, state) = make_state();
    let path = state
        .music_dir
        .join("aikko, katanacss, INSPACE, playingtheange - song.mp3");
    create_mp3(&path, "song", "aikko; katanacss; INSPACE; playingtheange");
    scan_music_dir(state.clone()).await.unwrap();

    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET artist = ?1 WHERE file_path = ?2",
            params![
                "aikko,katanacss,INSPACE,playingtheange",
                path.to_string_lossy()
            ],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(
        tracks[0].artist,
        "aikko; katanacss; INSPACE; playingtheange"
    );
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
async fn scan_music_dir_preserves_track_identity_after_move() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).unwrap();
    let id = tracks[0].id.clone();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1, artist = ?2 WHERE id = ?3",
            params!["Kept Title", "Kept Artist", &id],
        )
        .unwrap();
    }

    let moved_path = state.music_dir.join("moved.flac");
    fs::rename(&path, &moved_path).unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let renamed_path = state.music_dir.join("Kept Artist - Kept Title.flac");
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, id);
    assert_eq!(tracks[0].title, "Kept Title");
    assert_eq!(tracks[0].artist, "Kept Artist");
    assert_eq!(tracks[0].file_path, renamed_path.to_string_lossy());
    assert!(!moved_path.exists());
    assert!(renamed_path.exists());
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
    assert_eq!(tracks[0].id, tracks[0].content_hash);
}

#[tokio::test]
async fn scan_music_dir_standardizes_id3_tags() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.mp3");
    create_mp3(&path, "Tagged Song", "Artist");
    let mut tag = Tag::read_from_path(&path).unwrap();
    tag.set_title("Tagged Song");
    tag.write_to_path(&path, Version::Id3v24).unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).unwrap();
    let tag = Tag::read_from_path(&tracks[0].file_path).unwrap();
    assert_eq!(tag.version(), Version::Id3v23);
}
