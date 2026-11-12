use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use mus_backend::{db::init_db, state::AppState};
use rusqlite::Connection;
use tokio::sync::{broadcast, Mutex as AsyncMutex, Semaphore};

pub fn test_state(data_dir: PathBuf, mut conn: Connection) -> AppState {
    let music_dir = data_dir.join("music");
    let covers_dir = data_dir.join(".cache/covers");
    std::fs::create_dir_all(&music_dir).unwrap();
    std::fs::create_dir_all(&covers_dir).unwrap();
    init_db(&mut conn).unwrap();
    let (events, _) = broadcast::channel(100);
    AppState {
        db: Arc::new(Mutex::new(conn)),
        data_dir,
        static_dir: None,
        music_dir,
        covers_dir,
        events,
        download_lock: Arc::new(Semaphore::new(1)),
        mutation_locks: Arc::new(AsyncMutex::new(HashMap::new())),
        app_date: "2026-05-17".into(),
        commit_sha: Some("test-sha".into()),
        yt_dlp_version: Arc::new(AsyncMutex::new(Some("test-version".into()))),
    }
}
