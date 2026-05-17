use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{ExitStatus, Output},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use tokio::{process::Command, time};

use crate::error::AppError;

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "ogg", "wav"];

pub fn audio_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    visit_audio_paths(dir, &mut |path| paths.push(path.to_path_buf()))?;
    Ok(paths)
}

pub fn visit_audio_paths(dir: &Path, visit: &mut impl FnMut(&Path)) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_audio_paths(&path, visit)?;
        } else if is_audio_path(&path) {
            visit(&path);
        }
    }
    Ok(())
}

pub fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|v| v.to_str())
        .map(|v| {
            AUDIO_EXTENSIONS
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case(v))
        })
        .unwrap_or(false)
}

pub async fn command_output(command: &str, args: &[&str]) -> Result<String> {
    let output = command_output_with_timeout(command, args, Duration::from_secs(120)).await?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn command_output_text_with_timeout(
    command: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String> {
    let output = command_output_with_timeout(command, args, timeout).await?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn command_output_with_timeout(
    command: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output> {
    let mut process = Command::new(command);
    process.args(args).kill_on_drop(true);
    let output = time::timeout(timeout, process.output())
        .await
        .map_err(|_| anyhow!("{command} timed out"))??;
    if !output.status.success() {
        return Err(anyhow!(String::from_utf8_lossy(&output.stderr).to_string()));
    }
    Ok(output)
}

pub async fn run_command_output(mut command: Command, timeout: Duration) -> Result<Output> {
    command.kill_on_drop(true);
    time::timeout(timeout, command.output())
        .await
        .map_err(|_| anyhow!("command timed out"))?
        .map_err(Into::into)
}

pub async fn run_command_status(mut command: Command, timeout: Duration) -> Result<ExitStatus> {
    command.kill_on_drop(true);
    time::timeout(timeout, command.status())
        .await
        .map_err(|_| anyhow!("command timed out"))?
        .map_err(Into::into)
}

pub fn can_write(path: &Path) -> bool {
    let probe = path.join(format!(".mus-write-test-{}", now()));
    match fs::write(&probe, b"") {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

pub fn generate_filename(artist: &str, title: &str, ext: &str) -> Result<String, AppError> {
    let artists = artist
        .split(';')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let filename = format!(
        "{} - {}.{}",
        sanitize(&artists),
        sanitize(title),
        ext.trim_start_matches('.')
    );
    if filename.len() > 255 {
        return Err(AppError::bad_request("Filename too long"));
    }
    Ok(filename)
}

pub fn clean_title(raw: &str) -> String {
    raw.replace("Official Video", "")
        .replace("Official Audio", "")
        .trim_matches(|c| c == '-' || c == ' ' || c == '[' || c == ']')
        .trim()
        .to_string()
}

pub fn extract_artist_title(raw_title: &str, channel: &str) -> (String, String) {
    if let Some((artist, title)) = raw_title.split_once(" - ") {
        return (artist.trim().to_string(), clean_title(title));
    }
    (
        if channel.is_empty() {
            "Unknown Artist"
        } else {
            channel
        }
        .to_string(),
        clean_title(raw_title),
    )
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn system_time_secs(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|v| v.as_secs() as i64)
}

pub fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

pub fn file_content_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = 0xcbf29ce484222325u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in &buffer[..read] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("{hash:016x}"))
}

pub fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .collect()
}

#[cfg(unix)]
pub fn inode(meta: &fs::Metadata) -> Option<i64> {
    use std::os::unix::fs::MetadataExt;
    Some(meta.ino() as i64)
}

#[cfg(not(unix))]
pub fn inode(_: &fs::Metadata) -> Option<i64> {
    None
}
