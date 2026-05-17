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

    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 4_000);
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).unwrap();
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].updated_at, 4_000);
}

#[tokio::test]
async fn scan_music_dir_does_not_rewrite_audio_files() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.mp3");
    fs::write(&path, b"").unwrap();
    let mut tag = Tag::new();
    tag.set_title("Tagged Song");
    tag.write_to_path(&path, Version::Id3v24).unwrap();
    let before = fs::read(&path).unwrap();

    scan_music_dir(state.clone()).await.unwrap();

    assert_eq!(fs::read(&path).unwrap(), before);
}
