use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex as StdMutex, Weak},
};

use rusqlite::Connection;
use tokio::sync::{broadcast, Mutex as AsyncMutex, OwnedMutexGuard, Semaphore};

use crate::models::MusEvent;

/// Cloneable handles to application resources and configured storage paths.
///
/// Cloning shares connections, event channels, and locks; it does not copy the
/// database or media. Initialize the database before constructing the router.
#[derive(Clone)]
pub struct AppState {
    /// SQLite connection; runtime queries must use the database blocking worker.
    pub db: Arc<StdMutex<Connection>>,
    /// Root containing the separate `music` and `.cache` directories.
    pub data_dir: PathBuf,
    /// Optional built frontend assets served by the router fallback.
    pub static_dir: Option<PathBuf>,
    /// Music library mount; staging and recovery files must be stored elsewhere.
    pub music_dir: PathBuf,
    /// Generated WebP artwork under the app cache.
    pub covers_dir: PathBuf,
    /// Broadcast channel consumed by SSE clients.
    pub events: broadcast::Sender<MusEvent>,
    /// Owned-permit semaphore serializing downloads.
    pub download_lock: Arc<Semaphore>,
    /// Weak entries for keyed mutation locks; callers must retain acquired guards.
    pub mutation_locks: Arc<AsyncMutex<HashMap<String, Weak<AsyncMutex<()>>>>>,
    /// Build date shown in system information.
    pub app_date: String,
    /// Optional deployed source revision.
    pub commit_sha: Option<String>,
    /// Last successfully queried yt-dlp version, shared with the updater.
    pub yt_dlp_version: Arc<AsyncMutex<Option<String>>>,
}

impl AppState {
    /// Acquires an exclusive mutation lock for exactly this key.
    ///
    /// Keep the returned guard bound until the operation finishes. The registry
    /// lock is released before waiting for another operation with the same key.
    ///
    /// ```
    /// # async fn example(state: &mus_backend::state::AppState) {
    /// let _guard = state.mutation_lock("scanner".into()).await;
    /// // Perform the serialized operation while `_guard` remains in scope.
    /// # }
    /// ```
    pub async fn mutation_lock(&self, key: String) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.mutation_locks.lock().await;
            locks.retain(|_, lock| lock.strong_count() > 0);
            if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
                lock
            } else {
                let lock = Arc::new(AsyncMutex::new(()));
                locks.insert(key, Arc::downgrade(&lock));
                lock
            }
        };
        lock.lock_owned().await
    }
}
