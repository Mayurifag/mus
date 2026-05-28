use std::{env, path::Path, time::Duration};

use anyhow::Result;
use tokio::{fs::File, io::AsyncReadExt, time};

use crate::{db, state::AppState};

const DEFAULT_AUDIO_PREWARM_INTERVAL_SECONDS: u64 = 300;
const PREWARM_BUFFER_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy)]
enum PrewarmLimit {
    Disabled,
    Full,
    Bytes(u64),
}

impl PrewarmLimit {
    fn label(self) -> String {
        match self {
            Self::Disabled => "0".into(),
            Self::Full => "full".into(),
            Self::Bytes(bytes) => bytes.to_string(),
        }
    }
}

pub async fn run_prewarm_job(state: AppState) {
    loop {
        if let Err(error) = prewarm_music_dir(state.clone()).await {
            tracing::warn!("failed to prewarm music directory: {error}");
        }

        let interval = audio_prewarm_interval();
        if interval.is_zero() {
            break;
        }
        time::sleep(interval).await;
    }
}

pub async fn prewarm_music_dir(state: AppState) -> Result<()> {
    let limit = audio_prewarm_limit();
    if matches!(limit, PrewarmLimit::Disabled) {
        return Ok(());
    }

    let tracks = db::list_tracks(&state)?;
    let mut buffer = vec![0; buffer_size(limit)];
    let mut warmed = 0usize;
    let mut total_read = 0u64;
    let mut failed = 0usize;

    for track in tracks {
        let path = Path::new(&track.file_path);
        if !path.is_file() {
            continue;
        }

        match prewarm_path_bytes(path, limit, &mut buffer).await {
            Ok(read) => {
                warmed += 1;
                total_read += read;
            }
            Err(error) => {
                failed += 1;
                tracing::warn!(path = %track.file_path, "failed to prewarm audio file: {error}");
            }
        }
    }

    tracing::info!(warmed, failed, total_read, limit = %limit.label(), "audio prewarm completed");
    Ok(())
}

pub async fn prewarm_track_path(path: &Path) -> Result<u64> {
    let limit = audio_prewarm_limit();
    if matches!(limit, PrewarmLimit::Disabled) {
        return Ok(0);
    }

    let mut buffer = vec![0; buffer_size(limit)];
    prewarm_path_bytes(path, limit, &mut buffer).await
}

fn audio_prewarm_limit() -> PrewarmLimit {
    match env::var("AUDIO_PREWARM_BYTES") {
        Ok(value) if value.eq_ignore_ascii_case("full") || value.eq_ignore_ascii_case("all") => {
            PrewarmLimit::Full
        }
        Ok(value) => match value.parse::<u64>().ok() {
            Some(0) => PrewarmLimit::Disabled,
            Some(bytes) => PrewarmLimit::Bytes(bytes),
            None => PrewarmLimit::Full,
        },
        Err(_) => PrewarmLimit::Full,
    }
}

fn audio_prewarm_interval() -> Duration {
    Duration::from_secs(
        env::var("AUDIO_PREWARM_INTERVAL_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_AUDIO_PREWARM_INTERVAL_SECONDS),
    )
}

fn buffer_size(limit: PrewarmLimit) -> usize {
    match limit {
        PrewarmLimit::Disabled | PrewarmLimit::Full => PREWARM_BUFFER_BYTES as usize,
        PrewarmLimit::Bytes(bytes) => bytes.clamp(1, PREWARM_BUFFER_BYTES) as usize,
    }
}

async fn prewarm_path_bytes(path: &Path, limit: PrewarmLimit, buffer: &mut [u8]) -> Result<u64> {
    let mut file = File::open(path).await?;
    let mut remaining = match limit {
        PrewarmLimit::Disabled => Some(0),
        PrewarmLimit::Full => None,
        PrewarmLimit::Bytes(bytes) => Some(bytes),
    };
    let mut total_read = 0u64;

    loop {
        let read_size = match remaining {
            Some(0) => break,
            Some(bytes) => bytes.min(buffer.len() as u64) as usize,
            None => buffer.len(),
        };
        let read = file.read(&mut buffer[..read_size]).await?;
        if read == 0 {
            break;
        }
        if let Some(bytes) = remaining.as_mut() {
            *bytes -= read as u64;
        }
        total_read += read as u64;
    }

    Ok(total_read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn prewarm_path_reads_full_file() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let bytes = vec![7; (PREWARM_BUFFER_BYTES * 2 + 13) as usize];
        std::fs::write(temp.path(), &bytes).unwrap();
        let mut buffer = vec![0; buffer_size(PrewarmLimit::Full)];

        let read = prewarm_path_bytes(temp.path(), PrewarmLimit::Full, &mut buffer)
            .await
            .unwrap();

        assert_eq!(read, bytes.len() as u64);
    }

    #[tokio::test]
    async fn prewarm_path_respects_byte_limit() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), vec![7; 1024]).unwrap();
        let mut buffer = vec![0; buffer_size(PrewarmLimit::Bytes(9))];

        let read = prewarm_path_bytes(temp.path(), PrewarmLimit::Bytes(9), &mut buffer)
            .await
            .unwrap();

        assert_eq!(read, 9);
    }
}
