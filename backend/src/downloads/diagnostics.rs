use std::{process::Output, time::Duration};

use anyhow::{anyhow, Result};
use reqwest::Url;
use serde_json::{json, Value};
use tokio::process::Command;

use crate::util::{now_nanos, run_command_output};

pub(super) struct DownloadJob {
    id: String,
    video_id: Option<String>,
}

impl DownloadJob {
    pub(super) fn new(url: &str) -> Self {
        Self {
            id: format!("download-{}", now_nanos()),
            video_id: youtube_video_id(url),
        }
    }

    pub(super) fn span(&self, operation: &str) -> tracing::Span {
        tracing::info_span!(
            "downloader",
            job_id = self.id,
            video_id = self.video_id.as_deref().unwrap_or("unknown"),
            operation,
        )
    }

    pub(super) fn payload(&self) -> Value {
        json!({"job_id": self.id, "video_id": self.video_id})
    }
}

pub(super) fn youtube_video_id(input: &str) -> Option<String> {
    let url = Url::parse(input).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let id = match url.host_str()? {
        "youtu.be" | "www.youtu.be" => url.path().strip_prefix('/')?.to_string(),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            if url.path() == "/watch" {
                url.query_pairs()
                    .find(|(key, _)| key == "v")?
                    .1
                    .into_owned()
            } else {
                let mut segments = url.path_segments()?;
                match segments.next()? {
                    "shorts" | "embed" | "live" => segments.next()?.to_string(),
                    _ => return None,
                }
            }
        }
        _ => return None,
    };
    (id.len() == 11
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
    .then_some(id)
}

const RUNTIME_HINT: &str = "yt-dlp JavaScript runtime unavailable or challenge solving failed. Rebuild the backend image with Node >=22, the shared --js-runtimes node config, and bundled EJS scripts.";
const FORBIDDEN_HINT: &str = "yt-dlp HTTP 403: the source refused media access. Verify Node >=22 and --js-runtimes node, update the official nightly yt-dlp, then check source restrictions or configured cookies if it persists.";
const COMBINED_HINT: &str = "yt-dlp HTTP 403 with a JavaScript runtime/challenge failure. Rebuild the backend image with Node >=22, --js-runtimes node, and bundled EJS; if 403 persists with current nightly yt-dlp, check source restrictions or configured cookies.";
const TIMEOUT_HINT: &str = "yt-dlp timed out. Check source connectivity before trying again.";
const START_HINT: &str = "Unable to run yt-dlp. Check that yt-dlp and Python are installed.";
const FAILURE_HINT: &str =
    "Download failed. Check media tools, source access, and storage permissions.";

fn source_hint(stderr: &str) -> Option<&'static str> {
    let lower = stderr.to_ascii_lowercase();
    let runtime = lower.contains("javascript runtime")
        || lower.contains("js runtime")
        || lower.contains("challenge solving failed")
        || lower.contains("signature solving failed")
        || lower.contains("signature extraction failed");
    let forbidden = lower.contains("http error 403") || lower.contains("http 403");
    match (runtime, forbidden) {
        (true, true) => Some(COMBINED_HINT),
        (true, false) => Some(RUNTIME_HINT),
        (false, true) => Some(FORBIDDEN_HINT),
        _ => None,
    }
}

pub(super) fn safe_failure(error: &str) -> &'static str {
    // Only fixed messages cross the log/API/SSE boundary, never subprocess output or URLs.
    [
        RUNTIME_HINT,
        FORBIDDEN_HINT,
        COMBINED_HINT,
        TIMEOUT_HINT,
        START_HINT,
    ]
    .into_iter()
    .find(|message| *message == error)
    .unwrap_or(FAILURE_HINT)
}

pub(super) async fn yt_dlp_output(command: Command, timeout: Duration) -> Result<Output> {
    let output = run_command_output(command, timeout)
        .await
        .map_err(|error| {
            let message = if error.to_string() == "command timed out" {
                TIMEOUT_HINT
            } else {
                START_HINT
            };
            tracing::warn!(reason = message, "yt-dlp invocation failed");
            anyhow!(message)
        })?;
    let hint = source_hint(&String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        let message = hint.unwrap_or(FAILURE_HINT);
        tracing::warn!(
            exit_code = output.status.code(),
            reason = message,
            "yt-dlp failed"
        );
        return Err(anyhow!(message));
    }
    if let Some(message) = hint {
        tracing::warn!(reason = message, "yt-dlp warning");
    }
    Ok(output)
}

pub(super) fn safe_version(output: &Output) -> String {
    let version = String::from_utf8_lossy(&output.stdout);
    let version = version.trim();
    let number = version.strip_prefix('v').unwrap_or(version);
    if output.status.success()
        && !number.is_empty()
        && number.len() <= 64
        && number
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        version.to_string()
    } else {
        "unknown".into()
    }
}

pub(super) async fn log_versions() {
    let version = async |program| {
        let mut command = Command::new(program);
        command.arg("--version");
        run_command_output(command, Duration::from_secs(5))
            .await
            .map(|output| safe_version(&output))
            .unwrap_or_else(|_| "unavailable".into())
    };
    let (yt_dlp_version, node_version) = tokio::join!(version("yt-dlp"), version("node"));
    tracing::info!(yt_dlp_version, node_version, "downloader tool versions");
    if node_version
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .is_none_or(|major| major < 22)
    {
        tracing::warn!(reason = RUNTIME_HINT, "unsupported or missing Node runtime");
    }
}
