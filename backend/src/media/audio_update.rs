use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{anyhow, Result};
use filetime::{set_file_mtime, FileTime};
use tempfile::TempPath;
use tokio::process::Command;

use crate::models::AudioCut;
use crate::{
    audio_names::{normalize_artists, normalize_text},
    process::run_command_status,
};

use super::files::{allocate_path, move_path_and_touch, publish_updated_file};
use super::id3_tags::{needs_standardization, rewrite_to_temp, supports_id3};

mod cut;
use cut::cut_audio_to_temp;

/// Requested changes for one staged media rewrite and publication.
pub struct AudioUpdate<'a> {
    /// Replacement title; `None` preserves the existing tag.
    pub title: Option<&'a str>,
    /// Replacement artist credits; `None` preserves the existing tag.
    pub artist: Option<&'a str>,
    /// JPEG artwork to embed, or `None` to preserve existing artwork.
    pub cover_jpeg_path: Option<&'a Path>,
    /// Optional new path for the original file after editing.
    pub target_path: Option<&'a Path>,
    /// Standardize supported ID3 formats to v2.3 even without tag replacements.
    pub standardize_tags: bool,
    /// Set the resulting file's modification time to the time editing started.
    pub update_mtime: bool,
    /// Optional audio section to remove; lossy formats are re-encoded.
    pub remove_section: Option<AudioCut>,
}

/// Published path and changes made by [`apply_audio_update`].
pub struct AudioUpdateResult {
    /// Final path after any requested rename.
    pub path: PathBuf,
    /// Whether bytes or the path changed, not merely whether an update was requested.
    pub changed: bool,
    /// Whether bytes changed, requiring fingerprint/artwork/HLS refresh by the caller.
    pub content_changed: bool,
}

/// Stages edits outside the music directory and publishes them with recovery backup.
///
/// In-place rewrites preserve the original inode/xattrs. Cross-device moves copy
/// the file and cannot preserve that identity. Callers must serialize edits to the
/// same file and update database records and derived caches after publication.
/// Same-filesystem moves use atomic no-replace renames; unsupported filesystems
/// fail instead of falling back to an overwriting rename. Cross-device copies
/// reserve the destination with `create_new` before writing it.
///
/// # Errors
/// Returns invalid-cut, media-tool, worker, filesystem, unsupported-rename, or destination-conflict
/// errors. Unreadable existing ID3 tags abort rewrites rather than dropping
/// metadata; a missing ID3 tag is allowed. If restoring original bytes also fails,
/// the error names a retained
/// recovery backup; do not discard it before recovering the original.
///
/// # Panics
/// Panics if an edit needs runtime services but the future is polled without a
/// Tokio runtime with I/O and time enabled.
///
/// ```no_run
/// use std::path::Path;
/// use mus_backend::media::{apply_audio_update, AudioUpdate};
/// # async fn example() -> anyhow::Result<()> {
/// let result = apply_audio_update(Path::new("music/song.mp3"), Path::new(".cache"), AudioUpdate {
///     title: Some("New title"), artist: None, cover_jpeg_path: None,
///     target_path: None, standardize_tags: false, update_mtime: true, remove_section: None,
/// }).await?;
/// assert!(result.content_changed);
/// # Ok(())
/// # }
/// ```
pub async fn apply_audio_update(
    path: &Path,
    cache_dir: &Path,
    update: AudioUpdate<'_>,
) -> Result<AudioUpdateResult> {
    let title = update.title.map(normalize_text);
    let artist = update.artist.map(normalize_artists);
    let update = AudioUpdate {
        title: title.as_deref(),
        artist: artist.as_deref(),
        cover_jpeg_path: update.cover_jpeg_path,
        target_path: update.target_path,
        standardize_tags: update.standardize_tags,
        update_mtime: update.update_mtime,
        remove_section: update.remove_section,
    };
    let destination = update.target_path.unwrap_or(path);
    let path_changed = destination != path;
    let standardize_id3 = if update.standardize_tags {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || needs_standardization(&path)).await??
    } else {
        false
    };
    let media_changed = update.title.is_some()
        || update.artist.is_some()
        || update.cover_jpeg_path.is_some()
        || update.remove_section.is_some()
        || standardize_id3;

    if !path_changed && !media_changed {
        return Ok(AudioUpdateResult {
            path: path.to_path_buf(),
            changed: false,
            content_changed: false,
        });
    }

    let modified = update.update_mtime.then(FileTime::now);
    if media_changed {
        let tmp = if let Some(cut) = update.remove_section {
            cut_audio_to_temp(path, cache_dir, &update, cut).await?
        } else if supports_id3(path) {
            rewrite_to_temp(path, cache_dir, &update).await?
        } else {
            rewrite_audio_to_temp(path, cache_dir, &update).await?
        };
        let original = path.to_path_buf();
        let destination = destination.to_path_buf();
        let cache_dir = cache_dir.to_path_buf();
        tokio::task::spawn_blocking(move || {
            if let Some(modified) = modified {
                set_file_mtime(&tmp, modified)?;
            }
            publish_updated_file(&tmp, &original, &destination, modified, &cache_dir)
        })
        .await??;
    } else {
        let source = path.to_path_buf();
        let destination = destination.to_path_buf();
        tokio::task::spawn_blocking(move || move_path_and_touch(&source, &destination, modified))
            .await??;
    }

    Ok(AudioUpdateResult {
        path: destination.to_path_buf(),
        changed: true,
        content_changed: media_changed,
    })
}

/// Standardizes supported ID3 formats to v2.3; leaves other formats unchanged.
///
/// # Errors
/// Returns inspection, rewrite, publication, or worker errors. Uses the same
/// cache/recovery contract as [`apply_audio_update`]; see the [media example](crate::media).
///
/// # Panics
/// Has the runtime requirements of [`apply_audio_update`].
pub async fn standardize_audio_tags(path: &Path, cache_dir: &Path) -> Result<()> {
    apply_audio_update(
        path,
        cache_dir,
        AudioUpdate {
            title: None,
            artist: None,
            cover_jpeg_path: None,
            target_path: None,
            standardize_tags: true,
            update_mtime: true,
            remove_section: None,
        },
    )
    .await?;
    Ok(())
}

/// Replaces title/artist tags while preserving other tags and artwork.
///
/// # Errors
/// Returns rewrite, publication, or worker errors. Uses the same cache/recovery
/// contract as [`apply_audio_update`]; see the [media example](crate::media).
///
/// # Panics
/// Has the runtime requirements of [`apply_audio_update`].
pub async fn write_audio_tags(
    path: &Path,
    cache_dir: &Path,
    title: &str,
    artist: &str,
) -> Result<()> {
    apply_audio_update(
        path,
        cache_dir,
        AudioUpdate {
            title: Some(title),
            artist: Some(artist),
            cover_jpeg_path: None,
            target_path: None,
            standardize_tags: false,
            update_mtime: true,
            remove_section: None,
        },
    )
    .await?;
    Ok(())
}

/// Replaces embedded artwork with a JPEG while preserving existing text tags.
///
/// # Errors
/// Returns artwork-read, rewrite, publication, or worker errors. Uses the same
/// cache/recovery contract as [`apply_audio_update`]; see the [media example](crate::media).
///
/// # Panics
/// Has the runtime requirements of [`apply_audio_update`].
pub async fn write_audio_cover(path: &Path, cache_dir: &Path, jpeg_path: &Path) -> Result<()> {
    apply_audio_update(
        path,
        cache_dir,
        AudioUpdate {
            title: None,
            artist: None,
            cover_jpeg_path: Some(jpeg_path),
            target_path: None,
            standardize_tags: false,
            update_mtime: true,
            remove_section: None,
        },
    )
    .await?;
    Ok(())
}

async fn rewrite_audio_to_temp(
    path: &Path,
    cache_dir: &Path,
    update: &AudioUpdate<'_>,
) -> Result<TempPath> {
    let tmp = staging_file(cache_dir, "audio", path).await?;
    let mut command = Command::new("ffmpeg");
    command.args(["-y", "-i"]).arg(path);
    if let Some(jpeg_path) = update.cover_jpeg_path {
        command.args(["-i"]).arg(jpeg_path);
    }
    command.args(["-map", "0", "-map_metadata", "0", "-c", "copy"]);
    if update.cover_jpeg_path.is_some() {
        command.args([
            "-map",
            "1",
            "-c:v",
            "mjpeg",
            "-disposition:v",
            "attached_pic",
        ]);
    }
    if let Some(title) = update.title {
        command.arg("-metadata").arg(format!("title={title}"));
    }
    if let Some(artist) = update.artist {
        command.arg("-metadata").arg(format!("artist={artist}"));
    }
    command
        .arg(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = run_command_status(command, Duration::from_secs(300)).await?;
    if status.success()
        && tokio::fs::metadata(&tmp)
            .await
            .is_ok_and(|meta| meta.is_file())
    {
        Ok(tmp)
    } else {
        Err(anyhow!("ffmpeg failed to rewrite audio"))
    }
}

async fn staging_file(dir: &Path, kind: &'static str, path: &Path) -> Result<TempPath> {
    let dir = dir.to_path_buf();
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || allocate_path(&dir, kind, &path)).await?
}

#[cfg(all(test, unix))]
mod tests {
    use super::publish_updated_file;
    use std::{fs, io, path::PathBuf};
    use tempfile::TempDir;

    #[test]
    fn write_failure_restores_original() {
        if let Some(root) = std::env::var_os("MUS_WRITE_FAILURE_TEST") {
            let root = PathBuf::from(root);
            let original = root.join("original.mp3");
            let staged = root.join("cache/staged.mp3");
            let before = fs::read(&original).unwrap();
            let modified = fs::metadata(&original).unwrap().modified().unwrap();
            let error =
                publish_updated_file(&staged, &original, &original, None, &root.join("cache"))
                    .unwrap_err();
            assert_eq!(
                error.downcast_ref::<io::Error>().unwrap().kind(),
                io::ErrorKind::FileTooLarge
            );
            assert_eq!(fs::read(&original).unwrap(), before);
            assert_eq!(
                fs::metadata(&original).unwrap().modified().unwrap(),
                modified
            );
            assert_eq!(fs::read_dir(root.join("cache")).unwrap().count(), 1);
            return;
        }

        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("cache")).unwrap();
        fs::write(tmp.path().join("original.mp3"), [42; 128]).unwrap();
        fs::write(tmp.path().join("cache/staged.mp3"), [17; 8192]).unwrap();
        let output = std::process::Command::new("sh")
            .args(["-c", "trap '' XFSZ; ulimit -f 1; exec \"$@\"", "sh"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "media::audio_update::tests::write_failure_restores_original",
                "--nocapture",
            ])
            .env("MUS_WRITE_FAILURE_TEST", tmp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
