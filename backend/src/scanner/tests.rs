use super::*;
use crate::media::ProbeProgram;
use rusqlite::{params, Connection};

async fn assert_probe_failure_preserves_track(state: &AppState, id: &str) -> Vec<anyhow::Error> {
    let track = db::get_track(state, id).await.unwrap().unwrap();
    let path = PathBuf::from(&track.file_path);
    fs::write(&path, b"audio fixture").unwrap();
    assert!(db::set_download_tags(state, id, &path, &["ost".into()])
        .await
        .unwrap());
    let cover = state.covers_dir.join(format!("{id}_small.webp"));
    fs::write(&cover, b"cached cover").unwrap();
    let mut events = state.events.subscribe();
    let mut errors = Vec::new();

    // Cover full scans, changed files, and changed-directory traversal.
    for changed in [None, Some(path.clone()), Some(state.music_dir.clone())] {
        let result = match changed {
            Some(path) => sync_changed_paths(state.clone(), vec![path]).await,
            None => scan_music_dir(state.clone()).await,
        };
        assert!(
            db::get_track(state, id).await.unwrap().is_some(),
            "probe failure deleted the library record"
        );
        assert!(
            events.try_recv().is_err(),
            "probe failure broadcast a library change"
        );
        assert_eq!(fs::read(&path).unwrap(), b"audio fixture");
        assert_eq!(fs::read(&cover).unwrap(), b"cached cover");
        let track = db::get_track(state, id).await.unwrap().unwrap();
        assert_eq!(track.tags[0].name, "ost");
        assert!(db::track_tags_finalized(state, id).await.unwrap());
        errors.push(result.unwrap_err());
    }
    errors
}

#[tokio::test]
async fn missing_ffprobe_preserves_records_and_reports_the_start_error() {
    let (directory, state, id) = state_with_track(Path::new("Song.mp3"));
    let _probe = ProbeProgram::use_program(directory.path().join("missing-ffprobe"));
    for error in assert_probe_failure_preserves_track(&state, &id).await {
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::NotFound
        );
    }
}

#[cfg(unix)]
#[tokio::test(start_paused = true)]
async fn stalled_ffprobe_preserves_records_and_reports_the_timeout() {
    let (directory, state, id) = state_with_track(Path::new("Song.mp3"));
    let _probe = fake_probe(directory.path(), "exec /bin/sleep 60");
    for error in assert_probe_failure_preserves_track(&state, &id).await {
        assert!(error.is::<tokio::time::error::Elapsed>());
    }
}

#[cfg(unix)]
fn fake_probe(directory: &Path, script: &str) -> ProbeProgram {
    use std::os::unix::fs::PermissionsExt;

    let program = directory.join("ffprobe");
    fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    ProbeProgram::use_program(program)
}

#[cfg(unix)]
#[tokio::test]
async fn malformed_ffprobe_output_preserves_records_and_reports_the_parse_error() {
    let (directory, state, id) = state_with_track(Path::new("Song.mp3"));
    let _probe = fake_probe(directory.path(), "printf '%s' 'not json'");
    for error in assert_probe_failure_preserves_track(&state, &id).await {
        assert!(error.is::<serde_json::Error>());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn abnormal_ffprobe_exit_preserves_records_and_reports_the_failure() {
    let (directory, state, id) = state_with_track(Path::new("Song.mp3"));
    let _probe = fake_probe(directory.path(), "echo 'probe runtime failed' >&2; exit 2");
    for error in assert_probe_failure_preserves_track(&state, &id).await {
        assert!(format!("{error:#}").contains("probe runtime failed"));
    }
}

fn state_with_track(path: &Path) -> (tempfile::TempDir, AppState, String) {
    let directory = tempfile::TempDir::new().unwrap();
    let state = crate::test_support::test_state(
        directory.path().to_path_buf(),
        Connection::open_in_memory().unwrap(),
    );
    let id = "a".repeat(64);
    let file_path = state.music_dir.join(path);
    state.db.lock().unwrap().execute(
        "INSERT INTO track (id, title, artist, duration, file_path, added_at, updated_at, content_hash)
         VALUES (?1, 'Song', 'Artist', 1, ?2, 1, 1, ?1)",
        params![id, file_path.to_string_lossy()],
    ).unwrap();
    (directory, state, id)
}

#[tokio::test]
async fn removed_directory_removes_only_its_registered_tracks() {
    let (_directory, state, id) = state_with_track(Path::new("removed/Artist - Song.mp3"));
    let unrelated_id = "b".repeat(64);
    let unrelated_path = state.music_dir.join("removed-elsewhere/Song.mp3");
    state.db.lock().unwrap().execute(
        "INSERT INTO track (id, title, artist, duration, file_path, added_at, updated_at, content_hash)
         VALUES (?1, 'Other', 'Artist', 1, ?2, 1, 1, ?1)",
        params![unrelated_id, unrelated_path.to_string_lossy()],
    ).unwrap();
    let mut events = state.events.subscribe();
    let removed = state.music_dir.join("removed");

    sync_changed_paths(state.clone(), vec![removed])
        .await
        .unwrap();

    assert!(db::get_track(&state, &id).await.unwrap().is_none());
    assert!(db::get_track(&state, &unrelated_id)
        .await
        .unwrap()
        .is_some());
    let event = events.try_recv().unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_deleted"));
    assert_eq!(event.action_payload.unwrap()["id"], id);
    assert!(events.try_recv().is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn path_lookup_failures_do_not_delete_registered_tracks() {
    use std::os::unix::fs::symlink;

    let (_directory, state, id) = state_with_track(Path::new("album/Song.mp3"));
    let album = state.music_dir.join("album");
    symlink("album", &album).unwrap();
    let track_path = album.join("Song.mp3");
    let mut events = state.events.subscribe();

    for full_scan in [false, true] {
        let result = if full_scan {
            scan_music_dir(state.clone()).await
        } else {
            sync_changed_paths(state.clone(), vec![track_path.clone()]).await
        };
        let error = result.unwrap_err();
        assert!(error.chain().any(|cause| cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() != std::io::ErrorKind::NotFound)));
        assert!(db::get_track(&state, &id).await.unwrap().is_some());
        assert!(events.try_recv().is_err());
    }
}
