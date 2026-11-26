use std::{
    collections::HashSet,
    fs::{self, Metadata},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use futures_util::Stream;

use crate::files::is_audio_path;

type AudioFile = (PathBuf, Metadata);

pub(super) fn audio_paths_in_dir(dir: &Path) -> impl Stream<Item = Result<AudioFile>> + '_ {
    async_stream::try_stream! {
        let mut walk = AudioTree { pending: vec![dir.to_path_buf()], seen: HashSet::new() };
        // One blocking task per directory, not one per stat. Release the worker
        // while the scanner probes or registers audio files.
        while !walk.pending.is_empty() {
            let (next, files) = tokio::task::spawn_blocking(move || {
                let files = walk.read_next_directory();
                (walk, files)
            }).await?;
            walk = next;
            for file in files? {
                yield file;
            }
        }
    }
}

struct AudioTree {
    pending: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
}

impl AudioTree {
    fn read_next_directory(&mut self) -> Result<Vec<AudioFile>> {
        let mut files = Vec::new();
        while let Some(dir) = self.pending.pop() {
            let Some(canonical) = if_present(fs::canonicalize(&dir))
                .with_context(|| format!("failed to resolve directory {}", dir.display()))?
            else {
                continue;
            };
            if !self.seen.insert(canonical) {
                continue;
            }
            let Some(entries) = if_present(fs::read_dir(&dir))
                .with_context(|| format!("failed to read directory {}", dir.display()))?
            else {
                continue;
            };
            for entry in entries {
                let Some(entry) = if_present(entry).with_context(|| {
                    format!("failed to read directory entry in {}", dir.display())
                })?
                else {
                    continue;
                };
                let path = entry.path();
                let is_audio = is_audio_path(&path);
                if !is_audio {
                    let Some(kind) = if_present(entry.file_type())
                        .with_context(|| format!("failed to inspect path {}", path.display()))?
                    else {
                        continue;
                    };
                    if !kind.is_dir() && !kind.is_symlink() {
                        continue;
                    }
                }
                let Some(meta) = if_present(fs::metadata(&path))
                    .with_context(|| format!("failed to inspect path {}", path.display()))?
                else {
                    continue;
                };
                if meta.is_dir() {
                    self.pending.push(path);
                } else if is_audio && meta.is_file() {
                    files.push((path, meta));
                }
            }
            if !files.is_empty() {
                break;
            }
        }
        Ok(files)
    }
}

pub(super) fn missing_path(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::NotFound | ErrorKind::NotADirectory)
}

pub(super) fn if_present<T>(result: io::Result<T>) -> io::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if missing_path(error.kind()) => Ok(None),
        Err(error) => Err(error),
    }
}
