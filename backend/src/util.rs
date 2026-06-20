use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{ExitStatus, Output},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use tokio::{process::Command, time};
use unicode_normalization::UnicodeNormalization;

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
    path.is_dir() && can_write_dir(path)
}

#[cfg(unix)]
fn can_write_dir(path: &Path) -> bool {
    can_access(path, "-w") && can_access(path, "-x")
}

#[cfg(unix)]
fn can_access(path: &Path, flag: &str) -> bool {
    match std::process::Command::new("test")
        .arg(flag)
        .arg(path)
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn can_write_dir(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| !meta.permissions().readonly())
        .unwrap_or(false)
}

pub fn parse_artists(artist: &str) -> Vec<String> {
    artist
        .split([';', ','])
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(normalize_text)
        .collect()
}

pub fn normalize_artists(artist: &str) -> String {
    parse_artists(artist).join("; ")
}

pub fn generate_filename(artist: &str, title: &str, ext: &str) -> Result<String, AppError> {
    let artists = parse_artists(artist).join(", ");
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

pub fn normalize_text(value: &str) -> String {
    normalize_category_markers(&value.nfc().collect::<String>())
}

pub fn has_right_version_marker(value: &str) -> bool {
    let normalized = marker_match_text(value);
    normalized.contains("right version") || normalized.contains("right ver")
}

pub fn has_ai_cover_marker(value: &str) -> bool {
    let normalized = marker_match_text(value);
    normalized.contains("ai cover") || normalized.contains("a i cover")
}

pub fn has_gachi_marker(value: &str) -> bool {
    marker_match_text(value).contains("gachi")
}

pub fn inferred_tag_names(title: &str, artist: &str, path: &Path) -> Vec<String> {
    let haystack = format!("{title} {artist} {}", path.to_string_lossy());
    let mut tags = Vec::new();
    if has_gachi_marker(&haystack) || has_right_version_marker(&haystack) {
        tags.push("gachi".to_string());
    }
    if has_ai_cover_marker(&haystack) {
        tags.push("ai-cover".to_string());
    }
    tags
}

fn normalize_category_markers(value: &str) -> String {
    let has_right_version = has_right_version_marker(value);
    let has_ai_cover = has_ai_cover_marker(value);
    let mut output = value.to_string();
    for marker in RIGHT_VERSION_MARKERS {
        output = replace_ascii_insensitive(&output, marker, "");
    }
    for marker in AI_COVER_MARKERS {
        output = replace_ascii_insensitive(&output, marker, "");
    }
    output = output.split_whitespace().collect::<Vec<_>>().join(" ");
    if has_right_version {
        output = append_marker(output, "(Right version)");
    }
    if has_ai_cover {
        output = append_marker(output, "(AI cover)");
    }
    output
}

const RIGHT_VERSION_MARKERS: &[&str] = &[
    "(right version)",
    "[right version]",
    "right-version",
    "right version",
    "right ver",
];
const AI_COVER_MARKERS: &[&str] = &[
    "(ai cover)",
    "[ai cover]",
    "ai-cover",
    "a.i. cover",
    "ai cover",
];

fn marker_match_text(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace(['(', ')', '[', ']', '-', '_', '.'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn replace_ascii_insensitive(value: &str, needle: &str, replacement: &str) -> String {
    let lower_value = value.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut result = String::new();
    let mut index = 0;
    while let Some(relative_match) = lower_value[index..].find(&lower_needle) {
        let match_start = index + relative_match;
        let match_end = match_start + needle.len();
        result.push_str(&value[index..match_start]);
        result.push_str(replacement);
        index = match_end;
    }
    result.push_str(&value[index..]);
    result
}

fn append_marker(value: String, marker: &str) -> String {
    if value.is_empty() {
        marker.to_string()
    } else {
        format!("{value} {marker}")
    }
}

pub fn clean_title(raw: &str) -> String {
    normalize_text(
        raw.replace("Official Video", "")
            .replace("Official Audio", "")
            .trim_matches(|c| c == '-' || c == ' ' || c == '[' || c == ']')
            .trim(),
    )
}

pub fn extract_artist_title(raw_title: &str, channel: &str) -> (String, String) {
    if let Some((artist, title)) = raw_title.split_once(" - ") {
        return (normalize_artists(artist), clean_title(title));
    }
    (
        normalize_artists(if channel.is_empty() {
            "Unknown Artist"
        } else {
            channel
        }),
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
    let mut hash = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().to_hex().to_string())
}

pub fn is_hash_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn file_signature(meta: &fs::Metadata) -> String {
    let modified = meta
        .modified()
        .ok()
        .and_then(system_time_nanos)
        .unwrap_or_default();
    format!("{}:{modified}:{}", meta.len(), metadata_changed_nanos(meta))
}

fn system_time_nanos(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH).ok().map(|v| v.as_nanos())
}

#[cfg(unix)]
fn metadata_changed_nanos(meta: &fs::Metadata) -> i128 {
    use std::os::unix::fs::MetadataExt;
    i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec())
}

#[cfg(not(unix))]
fn metadata_changed_nanos(_: &fs::Metadata) -> i128 {
    0
}

pub fn sanitize(value: &str) -> String {
    normalize_text(value)
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        can_write, generate_filename, has_ai_cover_marker, has_right_version_marker,
        normalize_artists, normalize_text, parse_artists,
    };

    #[test]
    fn can_write_does_not_create_probe_files() {
        let tmp = TempDir::new().unwrap();

        assert!(can_write(tmp.path()));
        assert!(fs::read_dir(tmp.path()).unwrap().next().is_none());
    }

    #[test]
    fn parses_semicolon_and_comma_separated_artists() {
        assert_eq!(
            parse_artists("aikko,katanacss; INSPACE, playingtheangel"),
            vec!["aikko", "katanacss", "INSPACE", "playingtheangel"]
        );
    }

    #[test]
    fn normalizes_artists_for_storage() {
        assert_eq!(
            normalize_artists("aikko,katanacss,INSPACE"),
            "aikko; katanacss; INSPACE"
        );
    }

    #[test]
    fn generates_filename_with_readable_artist_spacing() {
        assert_eq!(
            generate_filename("aikko,katanacss,INSPACE", "song", "mp3").unwrap(),
            "aikko, katanacss, INSPACE - song.mp3"
        );
    }

    #[test]
    fn normalizes_unicode_to_nfc() {
        assert_eq!(normalize_text("Бедныи\u{0306}"), "Бедный");
        assert_eq!(
            generate_filename("Слава КПСС", "Бедныи\u{0306} Русскии\u{0306}", "mp3").unwrap(),
            "Слава КПСС - Бедный Русский.mp3"
        );
    }

    #[test]
    fn preserves_requested_artist_casing() {
        assert_eq!(normalize_artists("LIL KRYSTALLL"), "LIL KRYSTALLL");
        assert_eq!(
            generate_filename("LIL KRYSTALLL", "Air Force 1", "mp3").unwrap(),
            "LIL KRYSTALLL - Air Force 1.mp3"
        );
    }

    #[test]
    fn normalizes_right_version_markers() {
        for input in [
            "Song (right version)",
            "Song [right version]",
            "Song right ver",
            "Song right-version",
            "Song RIGHT VERSION",
        ] {
            assert_eq!(normalize_text(input), "Song (Right version)");
            assert!(has_right_version_marker(input));
        }
    }

    #[test]
    fn normalizes_ai_cover_markers() {
        for input in [
            "Song (ai cover)",
            "Song [ai cover]",
            "Song ai-cover",
            "Song AI COVER",
            "Song a.i. cover",
        ] {
            assert_eq!(normalize_text(input), "Song (AI cover)");
            assert!(has_ai_cover_marker(input));
        }
    }
}
