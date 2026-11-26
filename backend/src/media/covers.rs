//! Artwork extraction and ownership of its generated cache files.

use std::{fs::File, io::BufReader, path::Path, process::Stdio, time::Duration};

use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use image_webp::WebPDecoder;
use tempfile::TempPath;
#[cfg(not(all(test, unix)))]
use tokio::process::Command;

use crate::process::run_command_status;

#[cfg(all(test, unix))]
mod tests;

/// Successfully generated artwork, with a preview if the thumbnail could be decoded.
#[derive(Debug)]
pub struct GeneratedCover {
    /// Base64-encoded ThumbHash of the square thumbnail.
    pub thumbhash: Option<String>,
}

/// Generates original and 80×80 WebP artwork files named using a trusted track ID.
///
/// The caller must create `covers_dir` and supply a validated hash ID. Returns
/// `None` for missing/unconvertible artwork. Owns both output paths until
/// extraction completes; errors and cancellation trigger best-effort cleanup.
/// Existing outputs for this ID can be overwritten or removed during extraction.
///
/// # Errors
/// Returns path-resolution, subprocess-start, wait, timeout, or preview worker errors.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
///
/// ```no_run
/// # async fn example(path: &std::path::Path, covers: &std::path::Path, hash: &str) -> anyhow::Result<()> {
/// let cover = mus_backend::media::extract_cover(path, covers, hash).await?;
/// # let _ = cover;
/// # Ok(())
/// # }
/// ```
pub async fn extract_cover(
    path: &Path,
    covers_dir: &Path,
    id: &str,
) -> Result<Option<GeneratedCover>> {
    let mut original = TempPath::try_from_path(covers_dir.join(format!("{id}_original.webp")))?;
    let mut small = TempPath::try_from_path(covers_dir.join(format!("{id}_small.webp")))?;
    #[cfg(not(all(test, unix)))]
    let mut command = Command::new("ffmpeg");
    #[cfg(all(test, unix))]
    let mut command = tests::command();
    command
        // A single frame does not need FFmpeg's per-CPU filter pools.
        .args(["-y", "-filter_threads", "1", "-i"])
        .arg(path)
        .args(["-abort_on", "empty_output_stream"])
        .args(["-an", "-frames:v", "1", "-c:v", "libwebp", "-q:v", "85"])
        .arg(&original)
        .args([
            "-an",
            "-frames:v",
            "1",
            "-vf",
            "scale=80:80:force_original_aspect_ratio=increase,crop=80:80",
            "-c:v",
            "libwebp",
            "-q:v",
            "80",
        ])
        .arg(&small)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !run_command_status(command, Duration::from_secs(120))
        .await?
        .success()
    {
        return Ok(None);
    }
    let [original_path, small_path] = [&original, &small].map(|output| output.to_path_buf());
    let Some(cover) = tokio::task::spawn_blocking(move || {
        for output in [&original_path, &small_path] {
            if !std::fs::metadata(output).is_ok_and(|meta| meta.is_file() && meta.len() > 0) {
                return None;
            }
        }
        Some(GeneratedCover {
            thumbhash: read_cover_thumbhash(&small_path),
        })
    })
    .await?
    else {
        return Ok(None);
    };
    // Disarm the pair without fallible keep operations that could retain only one output.
    original.disable_cleanup(true);
    small.disable_cleanup(true);
    Ok(Some(cover))
}

pub(crate) fn read_cover_thumbhash(path: &Path) -> Option<String> {
    encode_cover_thumbhash(path)
        .inspect_err(|error| {
            tracing::debug!(path = %path.display(), %error, "could not generate cover preview");
        })
        .ok()
}

fn encode_cover_thumbhash(path: &Path) -> Result<String> {
    const SIZE: usize = 80;
    let mut decoder = WebPDecoder::new(BufReader::new(File::open(path)?))?;
    ensure!(
        decoder.dimensions() == (80, 80),
        "unexpected cover dimensions"
    );
    let mut rgba = vec![255; SIZE * SIZE * 4];
    if decoder.has_alpha() {
        decoder.read_image(&mut rgba)?;
    } else {
        let mut rgb = vec![0; SIZE * SIZE * 3];
        decoder.read_image(&mut rgb)?;
        for (target, pixel) in rgba
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(rgb.as_chunks::<3>().0)
        {
            target[..3].copy_from_slice(pixel);
        }
    }
    Ok(STANDARD.encode(thumbhash::rgba_to_thumb_hash(SIZE, SIZE, &rgba)))
}
