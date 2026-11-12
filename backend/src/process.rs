//! Subprocess helpers with deadlines and child cleanup on cancellation.
//!
//! Output helpers buffer the complete output. Callers must bound their workload
//! and choose a deadline appropriate for the media operation.
//!
//! ```no_run
//! use std::time::Duration;
//! use mus_backend::process::{command_output_text_with_timeout, run_command_output, run_command_status};
//! use tokio::process::Command;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let version = command_output_text_with_timeout("ffmpeg", &["-version"], Duration::from_secs(10)).await?;
//! let mut probe = Command::new("ffprobe");
//! probe.args(["-v", "error", "-show_format", "track.mp3"]);
//! let output = run_command_output(probe, Duration::from_secs(30)).await?;
//! assert!(output.status.success());
//! let mut update = Command::new("yt-dlp");
//! update.args(["--update-to", "nightly"]);
//! let status = run_command_status(update, Duration::from_secs(300)).await?;
//! assert!(status.success());
//! # Ok(())
//! # }
//! ```

use std::{
    process::{ExitStatus, Output},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use tokio::{process::Command, time};

/// Runs a successful command and returns trimmed, lossily decoded UTF-8 stdout.
///
/// # Errors
/// Returns an error if spawning, waiting, or reading output fails, the deadline
/// expires, or the command exits unsuccessfully. Nonzero exits use stderr as the message.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
pub async fn command_output_text_with_timeout(
    command: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String> {
    let output = command_output_with_timeout(command, args, timeout).await?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn command_output_with_timeout(
    command: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output> {
    let mut process = Command::new(command);
    process.args(args).kill_on_drop(true);
    let output = time::timeout(timeout, process.output())
        .await
        .with_context(|| format!("{command} timed out"))??;
    if !output.status.success() {
        return Err(anyhow!(String::from_utf8_lossy(&output.stderr).to_string()));
    }
    Ok(output)
}

/// Captures stdout, stderr, and exit status within the deadline.
///
/// A nonzero exit is returned in [`Output::status`], not converted to an error.
/// Dropping the operation or reaching its deadline kills the direct child;
/// it does not manage a process group or guarantee descendant termination.
///
/// # Errors
/// Returns an error if the deadline expires or the child cannot be spawned or awaited.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
pub async fn run_command_output(mut command: Command, timeout: Duration) -> Result<Output> {
    command.kill_on_drop(true);
    Ok(time::timeout(timeout, command.output())
        .await
        .context("command timed out")??)
}

/// Waits for a command within the deadline, honoring its configured stdio.
///
/// Nonzero exits are returned as statuses. Cancellation and timeout kill the
/// direct child only, as with [`run_command_output`].
///
/// # Errors
/// Returns an error if the deadline expires or the child cannot be spawned or awaited.
///
/// # Panics
/// Panics if polled without a Tokio runtime with I/O and time enabled.
pub async fn run_command_status(mut command: Command, timeout: Duration) -> Result<ExitStatus> {
    command.kill_on_drop(true);
    Ok(time::timeout(timeout, command.status())
        .await
        .context("command timed out")??)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_program_preserves_the_io_error() {
        let directory = tempfile::TempDir::new().unwrap();
        let command = Command::new(directory.path().join("missing-program"));
        let error = run_command_output(command, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::NotFound
        );
    }
}
