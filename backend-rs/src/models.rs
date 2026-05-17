use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub duration: i64,
    pub file_path: String,
    pub added_at: i64,
    pub updated_at: i64,
    pub has_cover: bool,
    pub inode: Option<i64>,
    pub content_hash: Option<String>,
    pub processing_status: String,
    pub last_error: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackDto {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub duration: i64,
    pub file_path: String,
    pub updated_at: i64,
    pub has_cover: bool,
    pub cover_small_url: Option<String>,
    pub cover_original_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub current_track_id: Option<i64>,
    pub progress_seconds: f64,
    pub volume_level: f64,
    pub is_muted: bool,
    pub is_shuffle: bool,
    pub is_repeat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusEvent {
    pub message_to_show: Option<String>,
    pub message_level: Option<String>,
    pub action_key: Option<String>,
    pub action_payload: Option<Value>,
}

#[derive(Deserialize)]
pub struct TrackUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub rename_file: Option<bool>,
}

#[derive(Deserialize)]
pub struct UrlRequest {
    pub url: String,
}

#[derive(Deserialize)]
pub struct ConfirmDownloadRequest {
    pub url: String,
    pub title: String,
    pub artist: String,
}

#[derive(Serialize)]
pub struct MetadataResponse {
    pub title: String,
    pub artist: String,
    pub thumbnail_url: Option<String>,
    pub duration: Option<f64>,
}
