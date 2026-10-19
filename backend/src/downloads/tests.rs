use std::{
    fs,
    io::{self, Write},
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    process::{ExitStatus, Output},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::Value;
use tempfile::TempDir;
use tokio::process::Command;
use tracing::{instrument::WithSubscriber, Instrument};

use super::diagnostics::{safe_failure, safe_version, yt_dlp_output, DownloadJob};

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
fn arbitrary_errors_and_version_output_are_not_exposed() {
    assert!(
        !safe_failure("https://user:private-password@example.com?token=private-token")
            .contains("private-")
    );
    let mut output = Output {
        status: ExitStatus::from_raw(0),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    for version in ["v22.17.0\n", "2026.10.19.123456\n"] {
        output.stdout = version.as_bytes().to_vec();
        assert_eq!(safe_version(&output), version.trim());
    }
    for version in [
        "badsecretversion",
        "private-token",
        "22.0.0\nprivate-token",
        "",
        "v",
    ] {
        output.stdout = version.as_bytes().to_vec();
        assert_eq!(safe_version(&output), "unknown");
    }
    output.status = ExitStatus::from_raw(1 << 8);
    output.stdout = b"2026.10.19.123456".to_vec();
    assert_eq!(safe_version(&output), "unknown");
}

#[tokio::test]
async fn failed_job_broadcasts_safe_correlated_events_and_releases_lock() {
    let tmp = TempDir::new().unwrap();
    let mut state = crate::app::test_state(
        tmp.path().to_path_buf(),
        rusqlite::Connection::open_in_memory().unwrap(),
    );
    // Fail before fetching media, with a private path in the underlying I/O error.
    state.data_dir = tmp.path().join("private-password");
    fs::write(&state.data_dir, b"not a directory").unwrap();
    let mut events = state.events.subscribe();
    super::spawn_download(
        state.clone(),
        "https://user:private-password@youtu.be/dQw4w9WgXcQ?token=private-token".into(),
        None,
        None,
        None,
        Vec::new(),
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
        super::run_download_inner(
            &state,
            command,
            None,
            None,
            Some(artwork_url),
            Vec::new(),
            &dir,
        )
        .await
    });
    let artwork_observed =
        tokio::time::timeout(Duration::from_secs(3), artwork_failed.notified()).await;
    let early = tokio::time::timeout(Duration::from_millis(100), &mut inner).await;
    let returned_early = early.is_ok();
    let worker_pending =
        tmp.path().join("media-started").exists() && !tmp.path().join("worker-finished").exists();
    // Release and wait before assertions, even if artwork incorrectly cancelled the media future.
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
