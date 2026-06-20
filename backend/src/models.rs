use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: i64,
    pub file_path: String,
    pub added_at: i64,
    pub updated_at: i64,
    pub has_cover: bool,
    pub inode: Option<i64>,
    pub file_signature: Option<String>,
    pub content_hash: String,
    pub processing_status: String,
    pub last_error: Option<Value>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackDto {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: i64,
    pub filename: String,
    pub updated_at: i64,
    pub has_cover: bool,
    pub cover_small_url: Option<String>,
    pub cover_original_url: Option<String>,
    pub hls_url: String,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub current_track_id: Option<String>,
    pub progress_seconds: f64,
    pub volume_level: f64,
    pub is_muted: bool,
    pub is_playing: bool,
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

#[derive(Clone, Deserialize)]
pub struct TrackUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub rename_file: Option<bool>,
    pub artwork_url: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ArtworkSearchQuery {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtworkSearchResult {
    pub id: String,
    pub source: String,
    pub title: String,
    pub artist: Option<String>,
    pub image_url: String,
    pub thumbnail_url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
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
    pub artwork_url: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct MetadataResponse {
    pub title: String,
    pub artist: String,
    pub thumbnail_url: Option<String>,
    pub duration: Option<f64>,
    pub tags: Vec<Tag>,
}

#[derive(Serialize)]
pub struct MusicDirectoryStatus {
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
    pub is_empty: Option<bool>,
    pub can_write: bool,
    pub warning: Option<String>,
}
