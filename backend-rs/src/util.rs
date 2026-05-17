use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use tokio::process::Command;

use crate::error::AppError;

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "ogg", "wav"];

pub fn audio_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    if !dir.is_dir() {
        return Ok(paths);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            paths.extend(audio_paths(&path)?);
        } else if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| AUDIO_EXTENSIONS.contains(&v.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub async fn command_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command).args(args).output().await?;
    if !output.status.success() {
        return Err(anyhow!(String::from_utf8_lossy(&output.stderr).to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
