//! Artwork extraction and ownership of its generated cache files.

use std::{path::Path, process::Stdio, time::Duration};

use anyhow::Result;
use tempfile::TempPath;
use tokio::fs;
#[cfg(not(all(test, unix)))]
use tokio::process::Command;

use crate::process::run_command_status;

#[cfg(all(test, unix))]
mod tests;

/// Generates original and 80×80 WebP artwork files named using a trusted track ID.
///
/// The caller must create `covers_dir` and supply a validated hash ID. Returns
/// `false` for missing/unconvertible artwork. Owns both output paths until both
/// conversions succeed; errors and cancellation trigger best-effort cleanup.
/// Existing outputs for this ID can be overwritten or removed during extraction.
///
/// # Errors
/// Returns path-resolution, subprocess-start, wait, or timeout errors.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
///
/// ```no_run
/// # async fn example(path: &std::path::Path, covers: &std::path::Path, hash: &str) -> anyhow::Result<()> {
/// let has_cover = mus_backend::media::extract_cover(path, covers, hash).await?;
/// # let _ = has_cover;
/// # Ok(())
/// # }
/// ```
pub async fn extract_cover(path: &Path, covers_dir: &Path, id: &str) -> Result<bool> {
    let mut original = TempPath::try_from_path(covers_dir.join(format!("{id}_original.webp")))?;
    let mut small = TempPath::try_from_path(covers_dir.join(format!("{id}_small.webp")))?;
    #[cfg(not(all(test, unix)))]
    let mut command = Command::new("ffmpeg");
    #[cfg(all(test, unix))]
    let mut command = tests::command();
    command
        .args(["-y", "-i"])
        .arg(path)
        .args(["-an", "-vframes", "1", "-vcodec", "libwebp", "-q:v", "85"])
        .arg(&original)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = run_command_status(command, Duration::from_secs(120)).await?;
    if !status.success()
        || !fs::metadata(&original)
            .await
            .is_ok_and(|meta| meta.is_file())
    {
        return Ok(false);
    }
    #[cfg(not(all(test, unix)))]
    let mut command = Command::new("ffmpeg");
    #[cfg(all(test, unix))]
    let mut command = tests::command();
    command
        .args(["-y", "-i"])
        .arg(&original)
        .args([
            "-vf",
            "scale=80:80:force_original_aspect_ratio=increase,crop=80:80",
            "-vcodec",
            "libwebp",
            "-q:v",
            "80",
        ])
        .arg(&small)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let small_status = run_command_status(command, Duration::from_secs(120)).await?;
    if !small_status.success() || !fs::metadata(&small).await.is_ok_and(|meta| meta.is_file()) {
        return Ok(false);
    }
    // Disarm the pair without fallible keep operations that could retain only one output.
    original.disable_cleanup(true);
    small.disable_cleanup(true);
    Ok(true)
}
