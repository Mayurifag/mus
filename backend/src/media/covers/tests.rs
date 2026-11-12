use std::{
    cell::RefCell,
    collections::VecDeque,
    fs,
    marker::PhantomData,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    rc::Rc,
};

use tokio::process::Command;

use super::extract_cover;

struct Program {
    path: PathBuf,
    pause_time: bool,
}

std::thread_local! {
    static PROGRAMS: RefCell<VecDeque<Program>> = const { RefCell::new(VecDeque::new()) };
}

struct Programs {
    previous: VecDeque<Program>,
    _thread: PhantomData<Rc<()>>,
}

impl Programs {
    fn install(programs: impl IntoIterator<Item = Program>) -> Self {
        Self {
            previous: PROGRAMS.with(|slot| slot.replace(programs.into_iter().collect())),
            _thread: PhantomData,
        }
    }

    fn assert_consumed(&self) {
        PROGRAMS.with(|slot| assert!(slot.borrow().is_empty()));
    }
}

impl Drop for Programs {
    fn drop(&mut self) {
        PROGRAMS.with(|slot| slot.replace(std::mem::take(&mut self.previous)));
    }
}

pub(super) fn command() -> Command {
    PROGRAMS.with(|slot| match slot.borrow_mut().pop_front() {
        Some(program) => {
            if program.pause_time {
                tokio::time::pause();
            }
            Command::new(program.path)
        }
        None => Command::new("ffmpeg"),
    })
}

fn script(directory: &Path, name: &str, ending: &str) -> PathBuf {
    let program = directory.join(name);
    fs::write(
        &program,
        format!(
            "#!/bin/sh\nfor output do :; done\nprintf 'generated cover' > \"$output\"\n{ending}\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program
}

fn seed_outputs(directory: &Path) {
    fs::write(
        directory.join("track_original.webp"),
        b"incomplete original",
    )
    .unwrap();
    fs::write(directory.join("track_small.webp"), b"incomplete thumbnail").unwrap();
    fs::write(directory.join("other_small.webp"), b"unrelated cover").unwrap();
    fs::write(directory.join("audio.mp3"), b"source audio").unwrap();
}

fn assert_cleanup(directory: &Path) {
    assert!(
        !directory.join("track_original.webp").exists(),
        "original output was not removed"
    );
    assert!(
        !directory.join("track_small.webp").exists(),
        "thumbnail output was not removed"
    );
    assert_eq!(
        fs::read(directory.join("other_small.webp")).unwrap(),
        b"unrelated cover"
    );
    assert_eq!(
        fs::read(directory.join("audio.mp3")).unwrap(),
        b"source audio"
    );
}

enum Stage {
    Original,
    Thumbnail,
}

async fn startup_failure_cleans_outputs(stage: Stage) {
    let directory = tempfile::TempDir::new().unwrap();
    seed_outputs(directory.path());
    let mut programs = Vec::new();
    if matches!(stage, Stage::Thumbnail) {
        programs.push(Program {
            path: script(directory.path(), "extract", "exit 0"),
            pause_time: false,
        });
    }
    programs.push(Program {
        path: directory.path().join("missing-ffmpeg"),
        pause_time: false,
    });
    let programs = Programs::install(programs);

    let error = extract_cover(
        &directory.path().join("audio.mp3"),
        directory.path(),
        "track",
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::NotFound
    );
    programs.assert_consumed();
    assert_cleanup(directory.path());
}

async fn timeout_cleans_outputs(stage: Stage) {
    let directory = tempfile::TempDir::new().unwrap();
    seed_outputs(directory.path());
    let mut programs = Vec::new();
    if matches!(stage, Stage::Thumbnail) {
        programs.push(Program {
            path: script(directory.path(), "extract", "exit 0"),
            pause_time: false,
        });
    }
    programs.push(Program {
        path: script(directory.path(), "stall", "exec /bin/sleep 300"),
        pause_time: true,
    });
    let programs = Programs::install(programs);

    let error = extract_cover(
        &directory.path().join("audio.mp3"),
        directory.path(),
        "track",
    )
    .await
    .unwrap_err();
    assert!(error.is::<tokio::time::error::Elapsed>());
    tokio::time::resume();
    programs.assert_consumed();
    assert_cleanup(directory.path());
}

#[tokio::test]
async fn original_startup_failure_cleans_outputs() {
    startup_failure_cleans_outputs(Stage::Original).await;
}

#[tokio::test]
async fn thumbnail_startup_failure_cleans_outputs() {
    startup_failure_cleans_outputs(Stage::Thumbnail).await;
}

#[tokio::test]
async fn original_timeout_cleans_outputs() {
    timeout_cleans_outputs(Stage::Original).await;
}

#[tokio::test]
async fn thumbnail_timeout_cleans_outputs() {
    timeout_cleans_outputs(Stage::Thumbnail).await;
}

#[tokio::test]
async fn cancellation_cleans_outputs() {
    let directory = tempfile::TempDir::new().unwrap();
    seed_outputs(directory.path());
    let programs = Programs::install([Program {
        path: script(directory.path(), "stall", "exec /bin/sleep 300"),
        pause_time: true,
    }]);
    let source = directory.path().join("audio.mp3");
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        extract_cover(&source, directory.path(), "track"),
    )
    .await;
    assert!(
        result.is_err(),
        "caller cancellation did not run before the extraction deadline"
    );
    tokio::time::resume();
    programs.assert_consumed();
    assert_cleanup(directory.path());
}

#[tokio::test]
async fn conversion_failure_still_returns_false_and_cleans_outputs() {
    for stage in [Stage::Original, Stage::Thumbnail] {
        let directory = tempfile::TempDir::new().unwrap();
        seed_outputs(directory.path());
        let mut programs = Vec::new();
        if matches!(stage, Stage::Thumbnail) {
            programs.push(Program {
                path: script(directory.path(), "extract", "exit 0"),
                pause_time: false,
            });
        }
        programs.push(Program {
            path: script(directory.path(), "reject", "exit 1"),
            pause_time: false,
        });
        let programs = Programs::install(programs);
        assert!(!extract_cover(
            &directory.path().join("audio.mp3"),
            directory.path(),
            "track"
        )
        .await
        .unwrap());
        programs.assert_consumed();
        assert_cleanup(directory.path());
    }
}
