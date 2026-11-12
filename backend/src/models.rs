//! Stored library data and audio edit ranges.
//!
//! ```
//! use mus_backend::models::{AudioCut, MusEvent, Tag};
//! let cut = AudioCut { start_seconds: 1.0, end_seconds: 2.0 };
//! mus_backend::media::validate_cut(cut, 10.0)?;
//! let tag = Tag { name: "ost".into(), display_name: "OST".into() };
//! let event = MusEvent {
//!     message_to_show: None,
//!     message_level: None,
//!     action_key: Some("track_updated".into()),
//!     action_payload: Some(serde_json::json!({"tags": [tag]})),
//! };
//! assert!(serde_json::to_string(&event)?.contains("track_updated"));
//! # Ok::<(), anyhow::Error>(())
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Database record returned by [`crate::db::list_tracks`] and [`crate::db::get_track`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    /// Library identifier, initially derived from the content hash.
    pub id: String,
    /// Stored title in Unicode NFC.
    pub title: String,
    /// Semicolon-separated artist credits, preserving commas within names.
    pub artist: String,
    /// Whole audio seconds; zero when duration is unavailable.
    pub duration: i64,
    /// Registered filesystem path, not an HTTP URL.
    pub file_path: String,
    /// Unix seconds when the track was first registered.
    pub added_at: i64,
    /// Unix seconds used for the last recorded file modification.
    pub updated_at: i64,
    /// Whether generated cover files are available.
    pub has_cover: bool,
    /// Unix inode when available and representable in the database.
    pub inode: Option<i64>,
    /// Optional metadata-based change hint, distinct from the content hash.
    pub file_signature: Option<String>,
    /// Recorded BLAKE3 content fingerprint and HLS cache key.
    pub content_hash: String,
    /// Processing state stored by the library workflow.
    pub processing_status: String,
    /// Optional structured processing failure.
    pub last_error: Option<Value>,
    /// Categories loaded in database tag order.
    pub tags: Vec<Tag>,
}

/// A category's stable machine name and display label.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tag {
    /// Machine name such as `right-version` or `ost`.
    pub name: String,
    /// User-facing label.
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrackDto {
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

impl From<Track> for TrackDto {
    fn from(track: Track) -> Self {
        let cover_small_url = track.has_cover.then(|| {
            format!(
                "/api/v1/tracks/{}/covers/small.webp?v={}",
                track.id, track.content_hash
            )
        });
        let cover_original_url = track.has_cover.then(|| {
            format!(
                "/api/v1/tracks/{}/covers/original.webp?v={}",
                track.id, track.content_hash
            )
        });
        Self {
            hls_url: crate::hls::track_hls_url(&track),
            id: track.id,
            title: track.title,
            artist: track.artist,
            duration: track.duration,
            filename: std::path::Path::new(&track.file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string(),
            updated_at: track.updated_at,
            has_cover: track.has_cover,
            cover_small_url,
            cover_original_url,
            tags: track.tags,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlayerState {
    pub current_track_id: Option<String>,
    pub progress_seconds: f64,
    pub volume_level: f64,
    pub is_muted: bool,
    pub is_playing: bool,
    pub is_shuffle: bool,
    pub is_repeat: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current_track_id: None,
            progress_seconds: 0.0,
            volume_level: 1.0,
            is_muted: false,
            is_playing: false,
            is_shuffle: false,
            is_repeat: false,
        }
    }
}

/// An SSE notification; absent fields serialize as JSON `null`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusEvent {
    /// Optional message to display to the user.
    pub message_to_show: Option<String>,
    /// Optional frontend notification level.
    pub message_level: Option<String>,
    /// Action identifying the frontend state update.
    pub action_key: Option<String>,
    /// Action-specific JSON data; keep secrets out of client-visible events.
    pub action_payload: Option<Value>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct TrackUpdate {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub rename_file: Option<bool>,
    pub artwork_url: Option<String>,
    pub tags: Option<Vec<String>>,
    pub remove_section: Option<AudioCut>,
}

/// A section to remove from an audio track, measured in seconds.
///
/// Validate against the source duration with [`crate::media::validate_cut`]
/// before building filters or writing audio.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct AudioCut {
    /// Inclusive start offset; must be finite and nonnegative.
    pub start_seconds: f64,
    /// End offset; must be finite, after the start, and within the source duration.
    pub end_seconds: f64,
}

#[derive(Deserialize)]
pub(crate) struct ArtworkSearchQuery {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ArtworkSearchResult {
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
pub(crate) struct UrlRequest {
    pub url: String,
}

#[derive(Deserialize)]
pub(crate) struct ConfirmDownloadRequest {
    pub url: String,
    pub title: String,
    pub artist: String,
    pub artwork_url: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub(crate) struct MetadataResponse {
    pub title: String,
    pub artist: String,
    pub thumbnail_url: Option<String>,
    pub duration: Option<f64>,
    pub tags: Vec<Tag>,
}

#[derive(Serialize)]
pub(crate) struct MusicDirectoryStatus {
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
    pub is_empty: Option<bool>,
    pub can_write: bool,
    pub warning: Option<String>,
}
