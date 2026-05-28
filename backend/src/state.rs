use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex as StdMutex},
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
    pub mutation_locks: Arc<AsyncMutex<HashMap<String, Arc<AsyncMutex<()>>>>>,
    pub app_date: String,
    pub commit_sha: Option<String>,
}

impl AppState {
    pub async fn mutation_lock(&self, key: String) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.mutation_locks.lock().await;
            locks
                .entry(key)
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };
        lock.lock_owned().await
    }
}
