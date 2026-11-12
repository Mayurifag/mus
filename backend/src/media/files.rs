//! Blocking file publication, including in-place writes and recovery backups.

use std::{fs, io, path::Path};

use anyhow::{anyhow, Result};
use filetime::{set_file_mtime, FileTime};
use tempfile::TempPath;

use crate::files::{path_exists_as_different_file, paths_are_same_entry};

pub(super) fn publish_updated_file(
    source: &Path,
    original: &Path,
    destination: &Path,
    modified: Option<FileTime>,
    cache_dir: &Path,
) -> Result<()> {
    if destination != original && path_exists_as_different_file(original, destination)? {
        return Err(anyhow!("A file with this name already exists"));
    }
    let original_mtime = FileTime::from_system_time(fs::metadata(original)?.modified()?);
    let mut output = fs::OpenOptions::new().write(true).open(original)?;
    let mut backup = allocate_path(cache_dir, "backup", original)?;
    copy_over(original, &backup)?;
    let result: Result<()> = (|| {
        // Keep the original inode and OpenCloud xattrs when replacing its bytes.
        write_contents(source, &mut output)?;
        if let Some(modified) = modified {
            set_file_mtime(original, modified)?;
        }
        if destination != original {
            move_path_and_touch(original, destination, None)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        if let Err(restore_error) = write_contents(&backup, &mut output)
            .and_then(|_| set_file_mtime(original, original_mtime).map_err(Into::into))
        {
            backup.disable_cleanup(true);
            let message = format!(
                "Audio update failed: {error}. Recovery failed: {restore_error}. Original backup retained at {}",
                backup.display()
            );
            return Err(error.context(message));
        }
        return Err(error);
    }
    let _ = fs::remove_file(source);
    Ok(())
}

pub(super) fn move_path_and_touch(
    source: &Path,
    destination: &Path,
    modified: Option<FileTime>,
) -> Result<()> {
    if source == destination {
        if let Some(modified) = modified {
            set_file_mtime(destination, modified)?;
        }
        return Ok(());
    }
    if path_exists_as_different_file(source, destination)? {
        return Err(anyhow!("A file with this name already exists"));
    }
    #[cfg(test)]
    tests::create_competing_destination();
    let result = match rename_without_replace(source, destination) {
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            copy_without_replace(source, destination).and_then(|()| fs::remove_file(source))
        }
        result => result,
    };
    if let Err(error) = result {
        if error.kind() == io::ErrorKind::AlreadyExists {
            // A rename between existing aliases was a no-op with fs::rename.
            // Never retry an overwriting rename, even after proving identity.
            if paths_are_same_entry(source, destination)? {
                if let Some(modified) = modified {
                    set_file_mtime(source, modified)?;
                }
                return Ok(());
            }
            return Err(anyhow::Error::new(error).context("A file with this name already exists"));
        }
        return Err(error.into());
    }
    if let Some(modified) = modified {
        set_file_mtime(destination, modified)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn rename_without_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags, CWD};

    // Use the syscall backend: Rust's bundled musl lacks the renameat2 symbol
    // required by renamore, even when the running kernel supports it.
    renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE).map_err(Into::into)
}

#[cfg(not(target_os = "linux"))]
fn rename_without_replace(source: &Path, destination: &Path) -> io::Result<()> {
    renamore::rename_exclusive(source, destination)
}

pub(super) fn copy_over(source: &Path, destination: &Path) -> Result<()> {
    let mut output = fs::OpenOptions::new().write(true).open(destination)?;
    write_contents(source, &mut output)
}

fn write_contents(source: &Path, output: &mut fs::File) -> Result<()> {
    use io::Seek;
    let mut input = fs::File::open(source)?;
    output.rewind()?;
    let length = io::copy(&mut input, output)?;
    output.set_len(length)?;
    output.sync_all()?;
    Ok(())
}

fn copy_without_replace(source: &Path, destination: &Path) -> io::Result<()> {
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    if let Err(error) = io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
        let _ = fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}

pub(super) fn allocate_path(dir: &Path, kind: &str, path: &Path) -> Result<TempPath> {
    fs::create_dir_all(dir)?;
    let suffix = path
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();
    Ok(tempfile::Builder::new()
        .prefix(&format!(".mus-{kind}-"))
        .suffix(&suffix)
        .tempfile_in(dir)?
        .into_temp_path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, io::Write, path::PathBuf};

    const COMPETING_BYTES: &[u8] = b"competing file";
    std::thread_local! {
        static CREATE_BEFORE_RENAME: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    struct DestinationCreation;

    impl DestinationCreation {
        fn before_rename(destination: &Path) -> Self {
            CREATE_BEFORE_RENAME.with(|slot| {
                assert!(slot.replace(Some(destination.to_path_buf())).is_none());
            });
            Self
        }
    }

    impl Drop for DestinationCreation {
        fn drop(&mut self) {
            let _ = CREATE_BEFORE_RENAME.try_with(|slot| {
                if let Ok(mut slot) = slot.try_borrow_mut() {
                    slot.take();
                }
            });
        }
    }

    pub(super) fn create_competing_destination() {
        let destination = CREATE_BEFORE_RENAME.with(|slot| slot.borrow_mut().take());
        if let Some(destination) = destination {
            fs::File::create_new(destination)
                .unwrap()
                .write_all(COMPETING_BYTES)
                .unwrap();
        }
    }

    #[test]
    fn move_rejects_destination_created_after_conflict_check() {
        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        fs::write(&source, b"original").unwrap();
        let _race = DestinationCreation::before_rename(&destination);

        let error = move_path_and_touch(&source, &destination, None).unwrap_err();
        assert_eq!(error.to_string(), "A file with this name already exists");
        assert_eq!(fs::read(&source).unwrap(), b"original");
        assert_eq!(fs::read(&destination).unwrap(), COMPETING_BYTES);
    }

    #[test]
    fn rewrite_restores_original_when_destination_created_after_conflict_check() {
        let directory = tempfile::TempDir::new().unwrap();
        let cache = directory.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let staged = cache.join("staged.mp3");
        let original = directory.path().join("original.mp3");
        let destination = directory.path().join("destination.mp3");
        fs::write(&staged, b"edited audio").unwrap();
        fs::write(&original, b"original").unwrap();
        let modified = FileTime::from_unix_time(1_700_000_000, 0);
        set_file_mtime(&original, modified).unwrap();
        let original_metadata = fs::metadata(&original).unwrap();
        let _race = DestinationCreation::before_rename(&destination);

        let error =
            publish_updated_file(&staged, &original, &destination, None, &cache).unwrap_err();
        assert_eq!(error.to_string(), "A file with this name already exists");
        assert_eq!(fs::read(&original).unwrap(), b"original");
        assert_eq!(fs::read(&destination).unwrap(), COMPETING_BYTES);
        assert_eq!(fs::read(&staged).unwrap(), b"edited audio");
        assert_eq!(
            fs::metadata(&original).unwrap().modified().unwrap(),
            original_metadata.modified().unwrap()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(
                fs::metadata(&original).unwrap().ino(),
                original_metadata.ino()
            );
        }
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn move_preserves_destination_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        let absent = directory.path().join("absent.mp3");
        fs::write(&source, b"original").unwrap();

        for target in [&source, &absent] {
            symlink(target, &destination).unwrap();
            let error = move_path_and_touch(&source, &destination, None).unwrap_err();
            assert_eq!(error.to_string(), "A file with this name already exists");
            assert_eq!(fs::read(&source).unwrap(), b"original");
            assert_eq!(fs::read_link(&destination).unwrap(), *target);
            fs::remove_file(&destination).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn move_between_existing_hard_links_remains_a_no_op() {
        use std::os::unix::fs::MetadataExt;

        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        fs::write(&source, b"original").unwrap();
        fs::hard_link(&source, &destination).unwrap();
        let original_inode = fs::metadata(&source).unwrap().ino();
        let modified = FileTime::from_unix_time(1_700_000_000, 0);

        move_path_and_touch(&source, &destination, Some(modified)).unwrap();
        assert_eq!(fs::read(&source).unwrap(), b"original");
        assert_eq!(fs::read(&destination).unwrap(), b"original");
        assert_eq!(fs::metadata(&source).unwrap().ino(), original_inode);
        assert_eq!(fs::metadata(&destination).unwrap().ino(), original_inode);
        assert_eq!(
            FileTime::from_last_modification_time(&fs::metadata(&source).unwrap()),
            modified
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn recovery_failure_retains_backup_and_original_io_cause() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::TempDir::new().unwrap();
        let cache = directory.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let staged = cache.join("staged.mp3");
        let original = directory.path().join("original.mp3");
        fs::write(&staged, b"edited audio").unwrap();
        // A device rejects set_len during both publication and recovery.
        symlink("/dev/null", &original).unwrap();

        let error = publish_updated_file(&staged, &original, &original, None, &cache).unwrap_err();
        let backups: Vec<_> = fs::read_dir(&cache)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".mus-backup-")
            })
            .collect();
        assert_eq!(backups.len(), 1);
        let message = error.to_string();
        assert!(message.contains("Recovery failed:"), "{message}");
        assert!(message.contains(&backups[0].display().to_string()));
        assert_eq!(fs::metadata(&backups[0]).unwrap().len(), 0);
        assert_eq!(fs::read(&staged).unwrap(), b"edited audio");
        assert_eq!(fs::read_link(&original).unwrap(), Path::new("/dev/null"));
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rewrite_then_rename_preserves_inode_and_xattrs() {
        use std::{os::unix::fs::MetadataExt, process::Command};

        let directory = tempfile::TempDir::new().unwrap();
        let cache = directory.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let staged = cache.join("staged.mp3");
        let original = directory.path().join("original.mp3");
        let destination = directory.path().join("destination.mp3");
        fs::write(&original, b"original").unwrap();
        fs::write(&staged, b"edited audio").unwrap();
        let original_metadata = fs::metadata(&original).unwrap();
        assert!(Command::new("python3")
            .args([
                "-c",
                "import os, sys; os.setxattr(sys.argv[1], 'user.mus-publication', b'preserve-me')"
            ])
            .arg(&original)
            .status()
            .unwrap()
            .success());

        publish_updated_file(&staged, &original, &destination, None, &cache).unwrap();
        assert!(!original.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"edited audio");
        let published_metadata = fs::metadata(&destination).unwrap();
        assert_eq!(published_metadata.dev(), original_metadata.dev());
        assert_eq!(published_metadata.ino(), original_metadata.ino());
        assert!(Command::new("python3")
            .args(["-c", "import os, sys; assert os.getxattr(sys.argv[1], 'user.mus-publication') == b'preserve-me'"])
            .arg(&destination).status().unwrap().success());
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cross_device_move_rejects_destination_created_after_conflict_check() {
        use std::os::unix::fs::MetadataExt;

        let source_directory = tempfile::TempDir::new_in("/dev/shm").unwrap();
        let destination_directory = tempfile::TempDir::new().unwrap();
        assert_ne!(
            fs::metadata(source_directory.path()).unwrap().dev(),
            fs::metadata(destination_directory.path()).unwrap().dev()
        );
        let source = source_directory.path().join("source.mp3");
        let destination = destination_directory.path().join("destination.mp3");
        fs::write(&source, b"original").unwrap();
        let _race = DestinationCreation::before_rename(&destination);

        let error = move_path_and_touch(&source, &destination, None).unwrap_err();
        assert_eq!(error.to_string(), "A file with this name already exists");
        assert_eq!(fs::read(&source).unwrap(), b"original");
        assert_eq!(fs::read(&destination).unwrap(), COMPETING_BYTES);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cross_device_move_to_absent_destination_preserves_contents() {
        use std::os::unix::fs::MetadataExt;

        let source_directory = tempfile::TempDir::new_in("/dev/shm").unwrap();
        let destination_directory = tempfile::TempDir::new().unwrap();
        assert_ne!(
            fs::metadata(source_directory.path()).unwrap().dev(),
            fs::metadata(destination_directory.path()).unwrap().dev()
        );
        let source = source_directory.path().join("source.mp3");
        let destination = destination_directory.path().join("destination.mp3");
        fs::write(&source, b"original").unwrap();

        move_path_and_touch(&source, &destination, None).unwrap();
        assert!(!source.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"original");
    }

    #[test]
    fn move_to_absent_destination_preserves_contents() {
        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        fs::write(&source, b"original").unwrap();

        move_path_and_touch(&source, &destination, None).unwrap();
        assert!(!source.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"original");
    }

    #[test]
    fn rewrite_in_place_preserves_the_path_and_removes_staging() {
        let directory = tempfile::TempDir::new().unwrap();
        let cache = directory.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let staged = cache.join("staged.mp3");
        let original = directory.path().join("original.mp3");
        fs::write(&staged, b"replaced").unwrap();
        fs::write(&original, b"original").unwrap();

        publish_updated_file(&staged, &original, &original, None, &cache).unwrap();
        assert_eq!(fs::read(&original).unwrap(), b"replaced");
        assert!(!staged.exists());
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
    }

    #[test]
    fn move_rejects_distinct_destination_with_matching_metadata() {
        let directory = tempfile::TempDir::new().unwrap();
        let source = directory.path().join("source.mp3");
        let destination = directory.path().join("destination.mp3");
        fs::write(&source, b"original").unwrap();
        fs::write(&destination, b"existing").unwrap();
        let modified = FileTime::from_unix_time(1_700_000_000, 0);
        set_file_mtime(&source, modified).unwrap();
        set_file_mtime(&destination, modified).unwrap();
        assert_eq!(
            fs::metadata(&source).unwrap().len(),
            fs::metadata(&destination).unwrap().len()
        );
        assert_eq!(
            fs::metadata(&source).unwrap().modified().unwrap(),
            fs::metadata(&destination).unwrap().modified().unwrap()
        );

        let error = move_path_and_touch(&source, &destination, None).unwrap_err();
        assert_eq!(error.to_string(), "A file with this name already exists");
        assert_eq!(fs::read(&source).unwrap(), b"original");
        assert_eq!(fs::read(&destination).unwrap(), b"existing");
    }

    #[test]
    fn rewrite_rejects_distinct_destination_with_matching_metadata() {
        let directory = tempfile::TempDir::new().unwrap();
        let cache = directory.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let staged = cache.join("staged.mp3");
        let original = directory.path().join("original.mp3");
        let destination = directory.path().join("destination.mp3");
        fs::write(&staged, b"replaced").unwrap();
        fs::write(&original, b"original").unwrap();
        fs::write(&destination, b"existing").unwrap();
        let modified = FileTime::from_unix_time(1_700_000_000, 0);
        set_file_mtime(&original, modified).unwrap();
        set_file_mtime(&destination, modified).unwrap();
        let original_meta = fs::metadata(&original).unwrap();
        let destination_meta = fs::metadata(&destination).unwrap();
        assert_eq!(original_meta.len(), destination_meta.len());
        assert_eq!(
            original_meta.modified().unwrap(),
            destination_meta.modified().unwrap()
        );

        let error = publish_updated_file(&staged, &original, &destination, Some(modified), &cache)
            .unwrap_err();
        assert_eq!(error.to_string(), "A file with this name already exists");
        assert_eq!(fs::read(&original).unwrap(), b"original");
        assert_eq!(fs::read(&destination).unwrap(), b"existing");
        assert_eq!(fs::read(&staged).unwrap(), b"replaced");
        assert_eq!(
            fs::metadata(&original).unwrap().modified().unwrap(),
            original_meta.modified().unwrap()
        );
        assert_eq!(
            fs::metadata(&destination).unwrap().modified().unwrap(),
            destination_meta.modified().unwrap()
        );
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
    }
}
