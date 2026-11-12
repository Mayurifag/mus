use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::process::Command;

use crate::{models::AudioCut, process::run_command_output};

/// Audio stream information used for cut validation and codec selection.
pub struct CutSource {
    /// Finite, positive source duration in seconds.
    pub duration: f64,
    /// ffprobe's audio codec name.
    pub codec: String,
}

/// Reads the first audio stream's codec and finite positive format duration.
///
/// # Errors
/// Returns errors for unavailable tools, timeout, malformed probe data, missing
/// audio streams, or unavailable duration. See the [media example](crate::media).
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
pub async fn read_cut_source(path: &Path) -> Result<CutSource> {
    let mut command = Command::new("ffprobe");
    command
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "format=duration:stream=codec_name",
            "-of",
            "json",
        ])
        .arg(path);
    let output = run_command_output(command, Duration::from_secs(30)).await?;
    if !output.status.success() {
        return Err(anyhow!("Unable to read audio for cutting"));
    }
    let data: Value = serde_json::from_slice(&output.stdout)?;
    let duration = data["format"]["duration"]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| anyhow!("Audio duration is unavailable"))?;
    let codec = data["streams"][0]["codec_name"]
        .as_str()
        .ok_or_else(|| anyhow!("No audio stream found"))?
        .to_string();
    Ok(CutSource { duration, codec })
}

/// Validates a removal range, retaining at least 0.1 seconds of audio.
///
/// Allows at most 0.001 seconds of end-offset tolerance for rounding.
///
/// # Errors
/// Returns an error for non-finite/nonpositive duration, non-finite offsets,
/// reversed/out-of-range offsets, or insufficient remaining audio.
///
/// ```
/// use mus_backend::{media::validate_cut, models::AudioCut};
/// validate_cut(AudioCut { start_seconds: 1.0, end_seconds: 2.0 }, 10.0)?;
/// assert!(validate_cut(AudioCut { start_seconds: 0.0, end_seconds: 10.0 }, 10.0).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn validate_cut(cut: AudioCut, duration: f64) -> Result<()> {
    if !duration.is_finite()
        || duration <= 0.0
        || !cut.start_seconds.is_finite()
        || !cut.end_seconds.is_finite()
        || cut.start_seconds < 0.0
        || cut.end_seconds <= cut.start_seconds
        || cut.end_seconds > duration + 0.001
        || duration - (cut.end_seconds - cut.start_seconds) < 0.1
    {
        return Err(anyhow!(
            "Choose a section within the track, leaving at least 0.1 seconds of audio"
        ));
    }
    Ok(())
}

pub(super) fn cut_filter(cut: AudioCut, duration: f64) -> String {
    removal_filter(cut, duration, 0.0, duration)
}

fn removal_filter(cut: AudioCut, duration: f64, before: f64, after: f64) -> String {
    let end = cut.end_seconds.min(duration);
    if cut.start_seconds == 0.0 {
        return trim_filter(end, after);
    }
    if end == duration {
        return trim_filter(before, cut.start_seconds);
    }
    format!(
        "[0:a:0]asplit=2[left][right];[left]atrim=start={before}:end={},asetpts=PTS-STARTPTS[a];[right]atrim=start={end}:end={after},asetpts=PTS-STARTPTS[b];[a][b]concat=n=2:v=0:a=1[out]",
        cut.start_seconds,
    )
}

fn trim_filter(start: f64, end: f64) -> String {
    format!("[0:a:0]atrim=start={start}:end={end},asetpts=PTS-STARTPTS[out]")
}

/// Which part of a proposed cut to preview without changing the source.
#[derive(Debug, Clone, Copy)]
pub enum CutPreview {
    /// Up to four seconds on either side of the join after removal.
    Join,
    /// Up to eight seconds of the section selected for removal.
    Selection,
}

/// Produces at most eight seconds of stereo WAV preview without modifying the source.
///
/// See [`CutPreview`] for the preview window and the [media example](crate::media).
///
/// # Errors
/// Returns invalid-range, subprocess, timeout, or invalid-output errors.
///
/// # Panics
/// Panics if valid input is processed without a Tokio runtime with I/O and time enabled.
pub async fn preview_cut(
    path: &Path,
    cut: AudioCut,
    duration: f64,
    preview: CutPreview,
) -> Result<Vec<u8>> {
    validate_cut(cut, duration)?;
    let filter = match preview {
        CutPreview::Selection => trim_filter(
            cut.start_seconds,
            cut.end_seconds.min(cut.start_seconds + 8.0),
        ),
        CutPreview::Join => removal_filter(
            cut,
            duration,
            (cut.start_seconds - 4.0).max(0.0),
            (cut.end_seconds.min(duration) + 4.0).min(duration),
        ),
    };
    let mut command = Command::new("ffmpeg");
    command
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[out]",
            "-vn",
            "-t",
            "8",
            "-ac",
            "2",
            "-ar",
            "44100",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
            "pipe:1",
        ])
        .stderr(Stdio::piped());
    let output = run_command_output(command, Duration::from_secs(60)).await?;
    if !output.status.success() || output.stdout.len() <= 44 {
        return Err(anyhow!("Unable to generate audio preview"));
    }
    Ok(output.stdout)
}
