//! Format metadata inspection and classification of ffprobe input rejections.

use std::{fmt, path::Path, process::Output, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::Value;
#[cfg(not(test))]
use tokio::process::Command;

use crate::{
    audio_names::{normalize_artists, normalize_text},
    process::run_command_output,
};

/// Normalized ffprobe metadata, with filename/unknown-artist fallbacks.
#[derive(Debug)]
pub struct MediaMetadata {
    /// Unicode NFC title, falling back to the filename stem.
    pub title: String,
    /// Semicolon-separated normalized credits, or `Unknown Artist`.
    pub artist: String,
    /// Truncated whole seconds; zero for unavailable or invalid duration.
    pub duration: i64,
    /// Whether an artist tag was present before fallback, including an empty tag.
    pub has_artist: bool,
}

/// ffprobe's ordinary failure exit, distinct from invocation/output failures.
#[derive(Debug)]
pub(crate) struct RejectedMedia(String);

impl fmt::Display for RejectedMedia {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RejectedMedia {}

/// Reads format metadata with a 30-second ffprobe deadline.
///
/// # Errors
/// Returns subprocess, timeout, or JSON parsing errors. Invalid duration values
/// use zero instead of failing the entire metadata read.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
pub async fn read_metadata(path: &Path) -> Result<MediaMetadata> {
    #[cfg(not(test))]
    let mut command = Command::new("ffprobe");
    #[cfg(test)]
    let mut command = tests::probe_command();
    command
        .args(["-v", "error", "-print_format", "json", "-show_format"])
        .arg(path);
    let output = run_command_output(command, Duration::from_secs(30)).await?;
    if !output.status.success() {
        let message = ffprobe_error_message(&output);
        // Preserve the scanner's skip policy for ffprobe's ordinary failure exit.
        // Crashes and other abnormal exits are not evidence of unparseable audio.
        return Err(if output.status.code() == Some(1) {
            RejectedMedia(message).into()
        } else {
            anyhow!(message)
        });
    }
    let data: Value = serde_json::from_slice(&output.stdout)?;
    let format = data.get("format").unwrap_or(&Value::Null);
    let tags = format.get("tags").unwrap_or(&Value::Null);
    let title = tags.get("title").and_then(Value::as_str);
    let artist = tags.get("artist").and_then(Value::as_str);
    Ok(MediaMetadata {
        title: normalize_text(title.unwrap_or_else(|| {
            path.file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("Unknown Title")
        })),
        artist: normalize_artists(artist.unwrap_or("Unknown Artist")),
        duration: format
            .get("duration")
            .and_then(Value::as_str)
            .and_then(parse_duration_seconds)
            .unwrap_or(0),
        has_artist: artist.is_some(),
    })
}

fn parse_duration_seconds(value: &str) -> Option<i64> {
    let seconds = value.parse::<f64>().ok()?;
    if seconds < 0.0 {
        return None;
    }
    let duration = Duration::try_from_secs_f64(seconds.trunc()).ok()?;
    i64::try_from(duration.as_secs()).ok()
}

fn ffprobe_error_message(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("ffprobe failed ({})", output.status)
    } else {
        format!("ffprobe failed ({}): {}", output.status, stderr)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::parse_duration_seconds;
    use std::{
        cell::RefCell,
        marker::PhantomData,
        path::{Path, PathBuf},
        rc::Rc,
    };
    use tokio::process::Command;

    std::thread_local! {
        static PROBE_PROGRAM: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    // Keep the override and its cleanup on the scanner test's current-thread runtime.
    pub(crate) struct ProbeProgram {
        previous: Option<PathBuf>,
        _thread: PhantomData<Rc<()>>,
    }

    impl ProbeProgram {
        pub(crate) fn use_program(program: PathBuf) -> Self {
            Self {
                previous: PROBE_PROGRAM.with(|slot| slot.replace(Some(program))),
                _thread: PhantomData,
            }
        }
    }

    impl Drop for ProbeProgram {
        fn drop(&mut self) {
            PROBE_PROGRAM.with(|slot| slot.replace(self.previous.take()));
        }
    }

    pub(super) fn probe_command() -> Command {
        PROBE_PROGRAM.with(|slot| {
            Command::new(
                slot.borrow()
                    .as_deref()
                    .unwrap_or_else(|| Path::new("ffprobe")),
            )
        })
    }

    #[test]
    fn duration_seconds_truncates_fractions_without_saturating_invalid_values() {
        for (input, expected) in [("123.75", 123), ("0.9999999999", 0), ("0", 0)] {
            assert_eq!(parse_duration_seconds(input), Some(expected));
        }
        for input in [
            "NaN",
            "inf",
            "-inf",
            "-1",
            "-0.5",
            "1e20",
            "9223372036854775808",
            "N/A",
        ] {
            assert_eq!(parse_duration_seconds(input), None, "{input}");
        }
    }
}
