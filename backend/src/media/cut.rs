use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::Value;
use tokio::process::Command;

use crate::{models::AudioCut, util::run_command_output};

pub struct CutSource {
    pub duration: f64,
    pub codec: String,
}

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

pub fn cut_filter(cut: AudioCut, duration: f64, preview: bool) -> String {
    let end = cut.end_seconds.min(duration);
    let before = if preview {
        (cut.start_seconds - 4.0).max(0.0)
    } else {
        0.0
    };
    let after = if preview {
        (end + 4.0).min(duration)
    } else {
        duration
    };
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

pub async fn preview_cut(
    path: &Path,
    cut: AudioCut,
    duration: f64,
    selection_only: bool,
) -> Result<Vec<u8>> {
    validate_cut(cut, duration)?;
    let filter = if selection_only {
        trim_filter(
            cut.start_seconds,
            cut.end_seconds.min(cut.start_seconds + 8.0),
        )
    } else {
        cut_filter(cut, duration, true)
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
