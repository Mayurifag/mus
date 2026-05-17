use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;
use tokio::sync::broadcast;

use crate::models::MusEvent;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub data_dir: PathBuf,
    pub static_dir: Option<PathBuf>,
    pub music_dir: PathBuf,
    pub covers_dir: PathBuf,
    pub events: broadcast::Sender<MusEvent>,
    pub download_lock: Arc<Mutex<bool>>,
    pub app_date: String,
    pub commit_sha: Option<String>,
}
