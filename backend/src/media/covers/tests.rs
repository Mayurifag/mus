use std::{
    cell::RefCell,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use tempfile::TempDir;
use tokio::process::Command;

use super::extract_cover;

thread_local! {
    static PROGRAM: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub(super) fn command() -> Command {
    PROGRAM.with(|slot| Command::new(slot.take().unwrap_or_else(|| "ffmpeg".into())))
}

fn seed_outputs(directory: &Path) {
    fs::write(directory.join("track_original.webp"), b"stale original").unwrap();
    fs::write(directory.join("track_small.webp"), b"stale thumbnail").unwrap();
    fs::write(directory.join("other_small.webp"), b"unrelated cover").unwrap();
    fs::write(directory.join("audio.mp3"), b"source audio").unwrap();
}

fn assert_cleanup(directory: &Path) {
    assert!(!directory.join("track_original.webp").exists());
    assert!(!directory.join("track_small.webp").exists());
    assert_eq!(
        fs::read(directory.join("other_small.webp")).unwrap(),
        b"unrelated cover"
    );
    assert_eq!(
        fs::read(directory.join("audio.mp3")).unwrap(),
        b"source audio"
    );
}

fn use_script(directory: &Path, body: &str) {
    let script = directory.join("ffmpeg");
    fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    PROGRAM.with(|slot| slot.replace(Some(script)));
}

#[tokio::test]
async fn startup_failure_cleans_outputs() {
    let directory = TempDir::new().unwrap();
    let path = directory.path();
    seed_outputs(path);
    PROGRAM.with(|slot| slot.replace(Some(path.join("missing-ffmpeg"))));

    let error = extract_cover(&path.join("audio.mp3"), path, "track")
        .await
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
    assert_cleanup(path);
}

#[tokio::test(start_paused = true)]
async fn timeout_cleans_outputs() {
    let directory = TempDir::new().unwrap();
    let path = directory.path();
    seed_outputs(path);
    use_script(path, "exec /bin/sleep 300");

    let error = extract_cover(&path.join("audio.mp3"), path, "track")
        .await
        .unwrap_err();
    assert!(error.is::<tokio::time::error::Elapsed>());
    assert_cleanup(path);
}

#[tokio::test(start_paused = true)]
async fn cancellation_cleans_outputs() {
    let directory = TempDir::new().unwrap();
    let path = directory.path();
    seed_outputs(path);
    use_script(path, "exec /bin/sleep 300");

    let source = path.join("audio.mp3");
    assert!(tokio::time::timeout(
        Duration::from_secs(1),
        extract_cover(&source, path, "track")
    )
    .await
    .is_err());
    assert_cleanup(path);
}

#[tokio::test]
async fn unsuccessful_or_empty_conversion_cleans_outputs() {
    for script in ["exit 1", "for output do :; done\n: > \"$output\""] {
        let directory = TempDir::new().unwrap();
        let path = directory.path();
        seed_outputs(path);
        use_script(path, script);

        assert!(extract_cover(&path.join("audio.mp3"), path, "track")
            .await
            .unwrap()
            .is_none());
        assert_cleanup(path);
    }
}
