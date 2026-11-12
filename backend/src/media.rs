//! Audio inspection and edits using ffprobe, ffmpeg, and ID3 tags.
//!
//! External tools must be available on `PATH`. Async operations require a Tokio
//! runtime with I/O and time enabled. Use a cache directory outside the music mount.
//!
//! ```no_run
//! use std::path::Path;
//! use mus_backend::media::{read_metadata, read_cut_source, validate_cut, preview_cut,
//!     standardize_audio_tags, write_audio_tags, write_audio_cover, CutPreview};
//! use mus_backend::models::AudioCut;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let path = Path::new("music/Artist - Song.wav");
//! let cache = Path::new(".cache");
//! let metadata = read_metadata(path).await?;
//! let source = read_cut_source(path).await?;
//! let cut = AudioCut { start_seconds: 1.0, end_seconds: 2.0 };
//! validate_cut(cut, source.duration)?;
//! let preview = preview_cut(path, cut, source.duration, CutPreview::Join).await?;
//! standardize_audio_tags(path, cache).await?;
//! write_audio_tags(path, cache, "Song", "Artist").await?;
//! write_audio_cover(path, cache, Path::new(".cache/cover.jpg")).await?;
//! # let _ = (metadata, preview);
//! # Ok(())
//! # }
//! ```

mod audio_update;
mod covers;
mod cut;
mod files;
pub(crate) mod id3_tags;
mod metadata;

pub use audio_update::{
    apply_audio_update, standardize_audio_tags, write_audio_cover, write_audio_tags, AudioUpdate,
    AudioUpdateResult,
};
pub use covers::extract_cover;
pub use cut::{preview_cut, read_cut_source, validate_cut, CutPreview, CutSource};
#[cfg(test)]
pub(crate) use metadata::tests::ProbeProgram;
pub(crate) use metadata::RejectedMedia;
pub use metadata::{read_metadata, MediaMetadata};
