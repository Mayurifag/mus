use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use futures_util::Stream;
use tokio::fs;

use crate::files::is_audio_path;

pub(super) fn audio_paths_in_dir(dir: &Path) -> impl Stream<Item = Result<PathBuf>> + '_ {
    async_stream::try_stream! {
        let mut dirs = vec![dir.to_path_buf()];
        let mut seen = HashSet::new();
        while let Some(dir) = dirs.pop() {
            match fs::metadata(&dir).await {
                Ok(meta) if meta.is_dir() => {}
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => Err(error).with_context(|| format!("failed to inspect directory {}", dir.display()))?,
            }
            let canonical = match fs::canonicalize(&dir).await {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => Err(error).with_context(|| format!("failed to resolve directory {}", dir.display()))?,
            };
            if !seen.insert(canonical) {
                continue;
            }
            let mut entries = match fs::read_dir(&dir).await {
                Ok(entries) => entries,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => Err(error).with_context(|| format!("failed to read directory {}", dir.display()))?,
            };
            loop {
                let entry = match entries.next_entry().await {
                    Ok(Some(entry)) => entry,
                    Ok(None) => break,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => Err(error).with_context(|| format!("failed to read directory entry in {}", dir.display()))?,
                };
                let path = entry.path();
                match fs::metadata(&path).await {
                    Ok(meta) if meta.is_dir() => dirs.push(path),
                    Ok(meta) if meta.is_file() && is_audio_path(&path) => yield path,
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => Err(error).with_context(|| format!("failed to inspect path {}", path.display()))?,
                }
            }
        }
    }
}
