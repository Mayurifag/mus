use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex as StdMutex, Weak},
};

use rusqlite::Connection;
use tokio::sync::{broadcast, Mutex as AsyncMutex, OwnedMutexGuard};

use crate::models::MusEvent;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<StdMutex<Connection>>,
    pub data_dir: PathBuf,
    pub static_dir: Option<PathBuf>,
    pub music_dir: PathBuf,
    pub covers_dir: PathBuf,
    pub events: broadcast::Sender<MusEvent>,
    pub download_lock: Arc<StdMutex<bool>>,
    pub mutation_locks: Arc<AsyncMutex<HashMap<String, Weak<AsyncMutex<()>>>>>,
    pub app_date: String,
    pub commit_sha: Option<String>,
    pub yt_dlp_version: Arc<StdMutex<Option<String>>>,
}

impl AppState {
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
