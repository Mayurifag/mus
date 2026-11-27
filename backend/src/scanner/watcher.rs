use std::{env, path::Path, time::Duration};

use notify::{
    event::{EventKind, ModifyKind, RemoveKind, RenameMode},
    Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher,
};
use tokio::{sync::mpsc, time};

use super::{paths::missing_path, sync_changed_paths};
use crate::{files::is_audio_path, state::AppState};

/// Watches the configured music directory and reconciles coalesced changes.
///
/// Uses polling when `WATCHFILES_FORCE_POLLING` is `1` or a supported spelling of
/// `true`. Logs setup and reconciliation failures instead of returning errors.
/// The watcher lives until this future ends or its task is cancelled.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
///
/// ```no_run
/// # async fn example(state: mus_backend::state::AppState) {
/// let watcher = tokio::spawn(mus_backend::scanner::watch_music_dir(state));
/// // Stop background watching when shutting down the application.
/// watcher.abort();
/// # }
/// ```
pub async fn watch_music_dir(state: AppState) {
    let (tx, rx) = mpsc::channel(100);
    let music_dir = state.music_dir.clone();
    let watcher = tokio::task::spawn_blocking(move || {
        let mut watcher = music_watcher(tx)?;
        watcher.watch(&music_dir, RecursiveMode::Recursive)?;
        Ok::<_, notify::Error>(watcher)
    })
    .await;
    let _watcher = match watcher {
        Ok(Ok(watcher)) => watcher,
        Ok(Err(error)) => {
            tracing::warn!("failed to start music directory watcher: {error}");
            return;
        }
        Err(error) => {
            tracing::warn!("music directory watcher worker failed: {error}");
            return;
        }
    };

    // Drop the receiver before the watcher to unblock a callback waiting to send.
    let mut rx = rx;
    while let Some(Some(event)) = state.shutdown.run_until_cancelled(rx.recv()).await {
        let event = match event {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!("failed to receive music directory event: {error}");
                continue;
            }
        };
        if !should_sync_event(&event).await {
            continue;
        }
        let mut paths = event.paths;
        if state
            .shutdown
            .run_until_cancelled(time::sleep(Duration::from_millis(500)))
            .await
            .is_none()
        {
            break;
        }
        while !state.shutdown.is_cancelled()
            && let Ok(event) = rx.try_recv()
        {
            match event {
                Ok(event) if should_sync_event(&event).await => paths.extend(event.paths),
                Ok(_) => {}
                Err(error) => tracing::warn!("failed to receive music directory event: {error}"),
            }
        }
        if state.shutdown.is_cancelled() {
            break;
        }
        if let Err(error) = sync_changed_paths(state.clone(), paths).await {
            tracing::warn!("failed to sync music directory: {error}");
        }
    }
}

enum MusicWatcher {
    Recommended(RecommendedWatcher),
    Poll(PollWatcher),
}

impl MusicWatcher {
    fn watch(&mut self, path: &Path, recursive_mode: RecursiveMode) -> notify::Result<()> {
        match self {
            Self::Recommended(watcher) => watcher.watch(path, recursive_mode),
            Self::Poll(watcher) => watcher.watch(path, recursive_mode),
        }
    }
}

fn music_watcher(tx: mpsc::Sender<notify::Result<Event>>) -> notify::Result<MusicWatcher> {
    let polling = env::var("WATCHFILES_FORCE_POLLING")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True"));
    let on_event = move |result| {
        let _ = tx.blocking_send(result);
    };
    if polling {
        PollWatcher::new(
            on_event,
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )
        .map(MusicWatcher::Poll)
    } else {
        RecommendedWatcher::new(on_event, Config::default()).map(MusicWatcher::Recommended)
    }
}

async fn should_sync_event(event: &Event) -> bool {
    if matches!(event.kind, EventKind::Remove(RemoveKind::Folder)) {
        return true;
    }
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return false;
    }
    let may_have_removed_directory = matches!(
        event.kind,
        EventKind::Remove(RemoveKind::Any)
            | EventKind::Modify(ModifyKind::Name(
                RenameMode::Any | RenameMode::Both | RenameMode::From
            ))
    );
    for path in &event.paths {
        if is_audio_path(path) {
            return true;
        }
        match tokio::fs::metadata(path).await {
            Ok(meta) if meta.is_dir() => return true,
            Err(error) if may_have_removed_directory && missing_path(error.kind()) => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, RemoveKind};

    #[tokio::test]
    async fn syncs_audio_and_directory_changes_but_not_access_or_unrelated_files() {
        let directory = tempfile::TempDir::new().unwrap();
        let event = |kind, path| Event::new(kind).add_path(path);
        assert!(
            should_sync_event(&event(
                EventKind::Create(CreateKind::Folder),
                directory.path().to_path_buf(),
            ))
            .await
        );
        for kind in [
            EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)),
            EventKind::Remove(RemoveKind::Any),
        ] {
            assert!(should_sync_event(&event(kind, directory.path().join("moved-album"))).await);
        }
        assert!(
            should_sync_event(&event(
                EventKind::Remove(RemoveKind::File),
                directory.path().join("removed.MP3"),
            ))
            .await
        );
        assert!(
            should_sync_event(&event(
                EventKind::Remove(RemoveKind::Folder),
                directory.path().join("removed-album"),
            ))
            .await
        );
        assert!(
            !should_sync_event(&event(
                EventKind::Access(AccessKind::Read),
                directory.path().join("song.mp3"),
            ))
            .await
        );
        assert!(
            !should_sync_event(&event(
                EventKind::Create(CreateKind::File),
                directory.path().join("notes.txt"),
            ))
            .await
        );
    }
}
