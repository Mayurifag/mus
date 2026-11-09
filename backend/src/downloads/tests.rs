use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::Value;
use tempfile::TempDir;
use tokio::process::Command;
use tracing::{instrument::WithSubscriber, Instrument};

use super::diagnostics::{safe_failure, yt_dlp_output, DownloadJob};

fn test_mp3(path: &std::path::Path, title: &str) {
    let output = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.1",
            "-c:a",
            "libmp3lame",
            "-metadata",
            &format!("title={title}"),
            "-metadata",
            "artist=Artist",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn copy_command(source: &std::path::Path, work: &std::path::Path) -> Command {
    let mut command = Command::new("cp");
    command.arg(source).arg(work.join("download.mp3"));
    command
}

#[tokio::test]
async fn download_preserves_acoustic_marker_in_file_and_library() {
    use id3::TagLike;

    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Original title");
    let parsed = super::parser::parse("Artist - Song (Acoustic) (Official Video)", "Artist", None);
    let path = super::run_download_inner(
        &state,
        copy_command(&source, &work),
        Some(parsed.title),
        Some(parsed.artist),
        None,
        Some(vec![]),
        &work,
    )
    .await
    .unwrap();

    assert_eq!(path.file_name().unwrap(), "Artist - Song (Acoustic).mp3");
    assert_eq!(
        id3::Tag::read_from_path(&path).unwrap().title(),
        Some("Song (Acoustic)")
    );
    assert_eq!(
        crate::db::list_tracks(&state).unwrap()[0].title,
        "Song (Acoustic)"
    );
}

#[tokio::test]
async fn unconfirmed_download_categories_are_inferred_after_registration_and_rename() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Song (cover)");
    let path = super::run_download_inner(
        &state,
        copy_command(&source, &work),
        None,
        None,
        None,
        None,
        &work,
    )
    .await
    .unwrap();
    assert_eq!(path.file_name().unwrap(), "Artist - Song (cover).mp3");
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert!(!crate::db::track_tags_finalized(&state, &track.id).unwrap());
    assert_eq!(
        track
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["cover"]
    );
    {
        let mut conn = state.db.lock().unwrap();
        crate::db::init_db(&mut conn).unwrap();
    }
    assert_eq!(
        crate::db::get_track(&state, &track.id)
            .unwrap()
            .unwrap()
            .tags[0]
            .name,
        "cover"
    );
}

#[tokio::test]
async fn confirmed_empty_categories_override_scanner_inference() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Song (cover)");
    super::run_download_inner(
        &state,
        copy_command(&source, &work),
        Some("Song (cover)".into()),
        Some("Artist".into()),
        None,
        Some(vec![]),
        &work,
    )
    .await
    .unwrap();
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert!(track.tags.is_empty());
    assert!(crate::db::track_tags_finalized(&state, &track.id).unwrap());
    {
        let mut conn = state.db.lock().unwrap();
        crate::db::init_db(&mut conn).unwrap();
    }
    assert!(crate::db::get_track(&state, &track.id)
        .unwrap()
        .unwrap()
        .tags
        .is_empty());
}

#[tokio::test]
async fn direct_download_does_not_keep_ambiguous_scanner_tags() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Discover");
    super::run_download_inner(
        &state,
        copy_command(&source, &work),
        None,
        None,
        None,
        None,
        &work,
    )
    .await
    .unwrap();
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert!(track.tags.is_empty());
    assert!(!crate::db::track_tags_finalized(&state, &track.id).unwrap());
    {
        let mut conn = state.db.lock().unwrap();
        crate::db::init_db(&mut conn).unwrap();
    }
    assert!(crate::db::get_track(&state, &track.id)
        .unwrap()
        .unwrap()
        .tags
        .is_empty());
}

#[tokio::test]
async fn direct_download_reinfers_categories_after_title_edit() {
    use axum::{
        extract::{Path, State},
        Json,
    };

    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Song");
    super::run_download_inner(
        &state,
        copy_command(&source, &work),
        None,
        None,
        None,
        None,
        &work,
    )
    .await
    .unwrap();
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert!(!crate::db::track_tags_finalized(&state, &track.id).unwrap());
    let update = crate::models::TrackUpdate {
        title: Some("Song (cover)".into()),
        artist: None,
        rename_file: None,
        artwork_url: None,
        tags: None,
        remove_section: None,
    };
    let _ = crate::tracks::update_track(State(state.clone()), Path(track.id), Json(update))
        .await
        .unwrap();
    let updated = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(track) = crate::db::list_tracks(&state)
                .unwrap()
                .into_iter()
                .find(|track| {
                    track.title == "Song (cover)"
                        && track.tags.iter().any(|tag| tag.name == "cover")
                })
            {
                break track;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        updated
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["cover"]
    );
    assert!(!crate::db::track_tags_finalized(&state, &updated.id).unwrap());
    crate::db::set_track_tags(&state, &updated.id, &[], false).unwrap();
    {
        let mut conn = state.db.lock().unwrap();
        crate::db::init_db(&mut conn).unwrap();
    }
    let restored = crate::db::get_track(&state, &updated.id).unwrap().unwrap();
    assert_eq!(
        restored
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["cover"]
    );
}

#[tokio::test]
async fn confirm_without_categories_does_not_finalize_inferred_tags() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Song (cover)");
    super::run_download_inner(
        &state,
        copy_command(&source, &work),
        Some("Song (cover)".into()),
        Some("Artist".into()),
        None,
        None,
        &work,
    )
    .await
    .unwrap();
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert!(!crate::db::track_tags_finalized(&state, &track.id).unwrap());
    assert_eq!(
        track
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["cover"]
    );
}

#[tokio::test]
async fn duplicate_upload_keeps_confirmed_categories() {
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
    };
    use tower::ServiceExt;

    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let folder = state.music_dir.join("nested");
    fs::create_dir_all(&folder).unwrap();
    let original = folder.join("Artist - Song (cover).mp3");
    test_mp3(&original, "Song (cover)");
    crate::media::write_audio_tags(
        &original,
        &state.data_dir.join(".cache"),
        "Song (cover)",
        "Artist",
    )
    .await
    .unwrap();
    crate::scanner::scan_music_dir(state.clone()).await.unwrap();
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert!(crate::db::set_download_tags(&state, &track.id, &original, &[]).unwrap());

    let boundary = "review-boundary";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nSong (cover)\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"artist\"\r\n\r\nArtist\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"copy.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\n").into_bytes();
    body.extend(fs::read(&original).unwrap());
    body.extend(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = crate::app::app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tracks/upload")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(crate::db::list_tracks(&state).unwrap().len(), 1);
    let saved = crate::db::get_track(&state, &track.id).unwrap().unwrap();
    assert!(saved.tags.is_empty());
    assert!(crate::db::track_tags_finalized(&state, &track.id).unwrap());
}

#[tokio::test]
async fn download_preserves_comma_in_artist_name() {
    use id3::TagLike;

    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Song");
    let path = super::run_download_inner(
        &state,
        copy_command(&source, &work),
        Some("Song".into()),
        Some("Tyler, The Creator; Guest".into()),
        None,
        Some(vec![]),
        &work,
    )
    .await
    .unwrap();
    assert_eq!(
        path.file_name().unwrap(),
        "Tyler, The Creator, Guest - Song.mp3"
    );
    assert_eq!(
        id3::Tag::read_from_path(&path).unwrap().artist(),
        Some("Tyler, The Creator; Guest")
    );
    assert_eq!(
        crate::db::list_tracks(&state).unwrap()[0].artist,
        "Tyler, The Creator; Guest"
    );
    crate::scanner::scan_music_dir(state.clone()).await.unwrap();
    {
        let mut conn = state.db.lock().unwrap();
        crate::db::init_db(&mut conn).unwrap();
    }
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert_eq!(track.artist, "Tyler, The Creator; Guest");
    assert_eq!(std::path::Path::new(&track.file_path), path);
}

#[tokio::test]
async fn confirmed_categories_replace_inferred_categories() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let source = tmp.path().join("source.mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&source, "Song (cover)");
    super::run_download_inner(
        &state,
        copy_command(&source, &work),
        Some("Song (cover)".into()),
        Some("Artist".into()),
        None,
        Some(vec!["ost".into()]),
        &work,
    )
    .await
    .unwrap();
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    assert_eq!(
        track
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["ost"]
    );
    {
        let mut conn = state.db.lock().unwrap();
        crate::db::init_db(&mut conn).unwrap();
    }
    assert_eq!(
        crate::db::get_track(&state, &track.id)
            .unwrap()
            .unwrap()
            .tags[0]
            .name,
        "ost"
    );
}

#[tokio::test]
async fn duplicate_download_does_not_change_existing_track_categories() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let original = state.music_dir.join("Artist - Song (cover).mp3");
    let work = tmp.path().join("work");
    fs::create_dir(&work).unwrap();
    test_mp3(&original, "Song (cover)");
    crate::scanner::scan_music_dir(state.clone()).await.unwrap();
    let track = crate::db::list_tracks(&state).unwrap().pop().unwrap();
    crate::db::set_track_tags(&state, &track.id, &["ost".into()], false).unwrap();
    let result = super::run_download_inner(
        &state,
        copy_command(&original, &work),
        None,
        None,
        None,
        Some(vec!["mashup".into()]),
        &work,
    )
    .await
    .unwrap();
    assert_eq!(result.file_name().unwrap(), "download.mp3");
    let current = crate::db::get_track(&state, &track.id).unwrap().unwrap();
    assert_eq!(
        current
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["ost"]
    );
    assert!(!crate::db::track_tags_finalized(&state, &track.id).unwrap());
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn job_payload_contains_only_job_and_validated_video_ids() {
    for url in [
        "https://youtube.com/watch?v=dQw4w9WgXcQ&token=private-token",
        "https://user:private-password@youtu.be/dQw4w9WgXcQ?si=private-token",
        "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://www.youtube.com/shorts/dQw4w9WgXcQ",
        "https://m.youtube.com/live/dQw4w9WgXcQ",
        "https://youtube.com/embed/dQw4w9WgXcQ",
    ] {
        let payload = DownloadJob::new(url).payload();
        assert_eq!(payload["video_id"], "dQw4w9WgXcQ");
        assert!(payload["job_id"].as_str().unwrap().starts_with("download-"));
        assert!(!payload.to_string().contains("private-"));
        assert_eq!(payload.as_object().unwrap().len(), 2);
    }
    for url in [
        "https://example.com/watch?v=dQw4w9WgXcQ&token=private-token",
        "https://youtube.com.evil.example/watch?v=dQw4w9WgXcQ",
        "https://youtube.com/watch?v=private-token",
        "https://youtube.com/watch?v=bad%0Avalue",
        "https://youtu.be/dQw4w9WgXcQ/extra",
        "--exec=private-token",
    ] {
        assert!(DownloadJob::new(url).payload()["video_id"].is_null());
    }
}

#[tokio::test]
async fn fake_tool_preserves_metadata_warnings_and_safe_download_failures() {
    let tmp = TempDir::new().unwrap();
    let executable = tmp.path().join("yt-dlp");
    fs::write(
        &executable,
        r#"#!/bin/sh
case "$1" in
    --dump-json)
        printf '%s\n' '{"id":"dQw4w9WgXcQ","title":"Song","artist":"Artist"}'
        printf '%s\n' 'WARNING: No supported JavaScript runtime could be found. token=private-token' >&2
        exit 0
        ;;
    --runtime-only)
        printf '%s\n' 'WARNING: No supported JavaScript runtime could be found. token=private-token' >&2
        ;;
    --http403)
        printf '%s\n' 'HTTP Error 403: https://example.com?token=private-token' >&2
        ;;
    --generic-error)
        printf '%s\n' 'ERROR: private-token private-password private-cookie' >&2
        ;;
    *)
        printf '%s\n' 'WARNING: No supported JavaScript runtime could be found.' >&2
        printf '%s\n' 'ERROR: HTTP Error 403: https://user:private-password@example.com/media?token=private-token Authorization: Bearer private-bearer Cookie: SID=private-cookie' >&2
        ;;
esac
exit 1
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let logs = LogBuffer::default();
    let writer = logs.clone();
    let subscriber = tracing::Dispatch::new(
        tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish(),
    );
    let job = DownloadJob::new("https://youtu.be/dQw4w9WgXcQ?token=private-token");
    let payload = job.payload();
    let span = tracing::dispatcher::with_default(&subscriber, || job.span("download"));
    async {
        let mut command = Command::new(&executable);
        command.args(["--dump-json", "--no-playlist", "--", "https://example.com"]);
        let output = yt_dlp_output(command, Duration::from_secs(2))
            .await
            .unwrap();
        let data: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(data["title"], "Song");

        for (arg, expected) in [
            ("--combined", &["HTTP 403", "Node >=22", "bundled EJS"][..]),
            ("--runtime-only", &["Node >=22", "bundled EJS"][..]),
            ("--http403", &["source refused media access"][..]),
            ("--generic-error", &["Check media tools"][..]),
        ] {
            let mut command = Command::new(&executable);
            command.arg(arg);
            let error = yt_dlp_output(command, Duration::from_secs(2))
                .await
                .unwrap_err();
            let message = safe_failure(&error.to_string());
            for &hint in expected {
                assert!(message.contains(hint), "{arg}: {message}");
            }
            assert!(!error.to_string().contains("private-"));
            let mut failed_payload = job.payload();
            failed_payload["error"] = message.into();
            assert!(!failed_payload.to_string().contains("private-"));
        }

        let missing = Command::new(tmp.path().join("does-not-exist"));
        let error = yt_dlp_output(missing, Duration::from_secs(2))
            .await
            .unwrap_err();
        assert!(safe_failure(&error.to_string()).contains("Python"));
        let mut slow = Command::new("sleep");
        slow.arg("5");
        let error = yt_dlp_output(slow, Duration::from_millis(10))
            .await
            .unwrap_err();
        assert!(safe_failure(&error.to_string()).contains("timed out"));
    }
    .instrument(span)
    .with_subscriber(subscriber)
    .await;
    let logs = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(logs.contains(payload["job_id"].as_str().unwrap()));
    assert!(logs.contains("dQw4w9WgXcQ"));
    assert!(logs.contains("yt-dlp warning"));
    assert!(logs.contains("yt-dlp failed"), "{logs}");
    assert!(logs.contains("exit_code"));
    for secret in ["private-", "https://", "Authorization:", "Cookie:"] {
        assert!(!logs.contains(secret), "leaked {secret}: {logs}");
    }
}

#[test]
fn arbitrary_errors_are_not_exposed() {
    assert!(
        !safe_failure("https://user:private-password@example.com?token=private-token")
            .contains("private-")
    );
}

#[tokio::test]
async fn failed_job_broadcasts_safe_correlated_events_and_releases_lock() {
    let tmp = TempDir::new().unwrap();
    let mut state = crate::app::test_state(
        tmp.path().to_path_buf(),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    state.data_dir = tmp.path().join("private-password");
    fs::write(&state.data_dir, b"not a directory").unwrap();
    let mut events = state.events.subscribe();
    super::spawn_download(
        state.clone(),
        "https://user:private-password@youtu.be/dQw4w9WgXcQ?token=private-token".into(),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let started = tokio::time::timeout(Duration::from_secs(15), events.recv())
        .await
        .unwrap()
        .unwrap();
    let failed = tokio::time::timeout(Duration::from_secs(15), events.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(started.action_key.as_deref(), Some("download_started"));
    assert_eq!(failed.action_key.as_deref(), Some("download_failed"));
    let started_payload = started.action_payload.as_ref().unwrap();
    let failed_payload = failed.action_payload.as_ref().unwrap();
    assert_eq!(started_payload["job_id"], failed_payload["job_id"]);
    assert_eq!(failed_payload["video_id"], "dQw4w9WgXcQ");
    for event in [started, failed] {
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(!serialized.contains("private-"));
        assert!(!serialized.contains("https://"));
    }
    assert!(!*state.download_lock.lock().unwrap());
}

#[tokio::test]
async fn artwork_failure_waits_for_media_and_its_child() {
    let tmp = TempDir::new().unwrap();
    let state = crate::app::test_state(
        tmp.path().join("data"),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    let mut command = Command::new("sh");
    command.current_dir(tmp.path()).args([
        "-c",
        r#"(
    i=0
    while [ ! -f release-media ] && [ "$i" -lt 500 ]; do
        sleep 0.01
        i=$((i + 1))
    done
    : > worker-finished
) </dev/null >/dev/null 2>&1 &
worker=$!
: > media-started
wait "$worker"
: > media-finished
"#,
    ]);
    let artwork_failed = Arc::new(tokio::sync::Notify::new());
    let notify = artwork_failed.clone();
    let dir = tmp.path().to_path_buf();
    let router = axum::Router::new().route(
        "/",
        axum::routing::get(move || {
            let notify = notify.clone();
            let dir = dir.clone();
            async move {
                let _ = tokio::time::timeout(Duration::from_secs(2), async {
                    while !dir.join("media-started").exists() {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                })
                .await;
                notify.notify_one();
                axum::http::StatusCode::BAD_GATEWAY
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let artwork_url = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let dir = tmp.path().to_path_buf();
    let mut inner = tokio::spawn(async move {
        super::run_download_inner(&state, command, None, None, Some(artwork_url), None, &dir).await
    });
    let artwork_observed =
        tokio::time::timeout(Duration::from_secs(3), artwork_failed.notified()).await;
    let early = tokio::time::timeout(Duration::from_millis(100), &mut inner).await;
    let returned_early = early.is_ok();
    let worker_pending =
        tmp.path().join("media-started").exists() && !tmp.path().join("worker-finished").exists();
    fs::write(tmp.path().join("release-media"), b"").unwrap();
    let result = match early {
        Ok(result) => Ok(result),
        Err(_) => tokio::time::timeout(Duration::from_secs(2), &mut inner).await,
    };
    let media_finished_before_return = tmp.path().join("media-finished").exists();
    let worker_finished = tokio::time::timeout(Duration::from_secs(6), async {
        while !tmp.path().join("worker-finished").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    if result.is_err() {
        inner.abort();
        let _ = inner.await;
    }
    server.abort();
    let _ = server.await;
    assert!(
        artwork_observed.is_ok(),
        "artwork never reached the endpoint"
    );
    assert!(worker_pending, "media worker was not pending");
    assert!(!returned_early, "artwork failure cancelled running media");
    assert!(worker_finished.is_ok(), "media child did not finish");
    assert!(
        media_finished_before_return,
        "media returned before finishing"
    );
    let error = result.unwrap().unwrap().unwrap_err();
    assert!(error.to_string().contains("502"), "{error}");
}
