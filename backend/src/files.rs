//! File discovery, BLAKE3 hashes, change signatures, and permission checks.
//!
//! Discovery and hashing perform blocking I/O; call them from a blocking worker
//! when using an async runtime. Signatures are change hints, not content hashes.
//!
//! ```
//! use mus_backend::files::{audio_paths, file_content_hash, file_signature, inode, is_audio_path, is_hash_id};
//! use std::path::Path;
//!
//! # fn example() -> anyhow::Result<()> {
//! assert!(is_audio_path(Path::new("song.MP3")));
//! let directory = tempfile::tempdir()?;
//! let path = directory.path().join("song.mp3");
//! std::fs::write(&path, b"audio fixture")?;
//! let metadata = std::fs::metadata(&path)?;
//! assert!(is_hash_id(&file_content_hash(&path)?));
//! assert!(!file_signature(&metadata).is_empty());
//! let _inode = inode(&metadata);
//! assert_eq!(audio_paths(directory.path())?, vec![path]);
//! # Ok(())
//! # }
//! # example()?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "ogg", "wav"];

/// Recursively lists supported audio paths in filesystem enumeration order.
///
/// Follows directory symlinks. A missing or non-directory root yields an empty list.
///
/// # Errors
/// Returns directory-reading errors. This function performs blocking I/O.
pub fn audio_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    visit_audio_paths(dir, &mut |path| paths.push(path.to_path_buf()))?;
    Ok(paths)
}

fn visit_audio_paths(dir: &Path, visit: &mut impl FnMut(&Path)) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            visit_audio_paths(&path, visit)?;
        } else if is_audio_path(&path) {
            visit(&path);
        }
    }
    Ok(())
}

/// Checks the extension, case-insensitively, without accessing the filesystem.
///
/// Supports MP3, FLAC, M4A, OGG, and WAV; this does not validate file contents.
pub fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            AUDIO_EXTENSIONS
                .iter()
                .any(|supported| supported.eq_ignore_ascii_case(ext))
        })
}

/// Checks directory write access without creating a probe file.
///
/// Unix also requires search access and an available `test` executable. Other
/// platforms check the read-only flag, not the complete ACL. Lookup/check failures
/// return `false`; a successful check does not reserve access for a later write.
///
/// # Panics
/// Panics if polled outside a Tokio runtime; Unix process checks also require I/O enabled.
///
/// ```
/// # async fn example() -> anyhow::Result<()> {
/// let directory = tempfile::tempdir()?;
/// let writable = mus_backend::files::can_write(directory.path()).await;
/// # let _ = writable;
/// # Ok(())
/// # }
/// ```
pub async fn can_write(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|meta| meta.is_dir())
        && can_write_dir(path).await
}

#[cfg(unix)]
async fn can_write_dir(path: &Path) -> bool {
    can_access(path, "-w").await && can_access(path, "-x").await
}

#[cfg(unix)]
async fn can_access(path: &Path, flag: &str) -> bool {
    tokio::process::Command::new("test")
        .arg(flag)
        .arg(path)
        .status()
        .await
        .is_ok_and(|status| status.success())
}

#[cfg(not(unix))]
async fn can_write_dir(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|meta| !meta.permissions().readonly())
}

/// Hashes all file bytes as lowercase BLAKE3 hexadecimal text.
///
/// # Errors
/// Returns file-open or read errors. This function performs blocking I/O.
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

/// Accepts exactly 64 ASCII hexadecimal characters, including uppercase letters.
pub fn is_hash_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn path_exists_as_different_file(
    source: &Path,
    destination: &Path,
) -> std::io::Result<bool> {
    let destination_meta = match fs::metadata(destination) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(!same_file_metadata(
        &fs::metadata(source)?,
        &destination_meta,
    ))
}

pub(crate) fn paths_are_same_entry(source: &Path, destination: &Path) -> std::io::Result<bool> {
    let destination_meta = match fs::symlink_metadata(destination) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(same_file_metadata(
        &fs::symlink_metadata(source)?,
        &destination_meta,
    ))
}

#[cfg(unix)]
fn same_file_metadata(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file_metadata(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    // Length and timestamps cannot prove identity. Without a reliable identity
    // implementation here, existing destinations must be treated as conflicts.
    false
}

/// Builds a change hint from size, modification time, and Unix change time.
///
/// Uses zero for unavailable/pre-epoch modification times and for change time on
/// non-Unix platforms. Reads only the supplied metadata, not the file contents.
pub fn file_signature(meta: &fs::Metadata) -> String {
    let modified = meta
        .modified()
        .ok()
        .and_then(system_time_nanos)
        .unwrap_or_default();
    format!("{}:{modified}:{}", meta.len(), metadata_changed_nanos(meta))
}

fn system_time_nanos(time: SystemTime) -> Option<u128> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_nanos())
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

#[cfg(unix)]
/// Returns the Unix inode if it fits in the database's signed 64-bit column.
pub fn inode(meta: &fs::Metadata) -> Option<i64> {
    use std::os::unix::fs::MetadataExt;
    i64::try_from(meta.ino()).ok()
}

#[cfg(not(unix))]
/// Returns `None` on platforms without the Unix inode representation.
pub fn inode(_: &fs::Metadata) -> Option<i64> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn distinct_files_with_matching_metadata_are_conflicts() {
        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        std::fs::write(&source, b"original").unwrap();
        std::fs::write(&destination, b"existing").unwrap();
        let modified = filetime::FileTime::from_unix_time(1_700_000_000, 0);
        filetime::set_file_mtime(&source, modified).unwrap();
        filetime::set_file_mtime(&destination, modified).unwrap();
        let source_meta = std::fs::metadata(&source).unwrap();
        let destination_meta = std::fs::metadata(&destination).unwrap();
        assert_eq!(source_meta.len(), destination_meta.len());
        assert_eq!(
            source_meta.modified().unwrap(),
            destination_meta.modified().unwrap()
        );
        assert!(super::path_exists_as_different_file(&source, &destination).unwrap());
    }

    #[test]
    fn absent_destination_is_not_a_conflict() {
        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        std::fs::write(&source, b"original").unwrap();
        assert!(!super::path_exists_as_different_file(&source, &destination).unwrap());
        assert!(!super::paths_are_same_entry(&source, &destination).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn unix_hard_link_identity_is_not_a_conflict() {
        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        std::fs::write(&source, b"original").unwrap();
        std::fs::hard_link(&source, &destination).unwrap();
        assert!(!super::path_exists_as_different_file(&source, &destination).unwrap());
        assert!(super::paths_are_same_entry(&source, &destination).unwrap());
    }

    #[tokio::test]
    async fn can_write_does_not_create_probe_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(super::can_write(tmp.path()).await);
        assert!(std::fs::read_dir(tmp.path()).unwrap().next().is_none());
    }
}
