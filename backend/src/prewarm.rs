use std::{env, path::Path, time::Duration};

use anyhow::Result;
use tokio::{fs::File, io::AsyncReadExt, time};

use crate::{db, state::AppState};

const DEFAULT_AUDIO_PREWARM_BYTES: u64 = 4 * 1024 * 1024;
const DEFAULT_AUDIO_PREWARM_INTERVAL_SECONDS: u64 = 300;
const PREWARM_BUFFER_BYTES: u64 = 64 * 1024;

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
    let bytes = audio_prewarm_bytes();
    if bytes == 0 {
        return Ok(());
    }

    let tracks = db::list_tracks(&state)?;
    let mut buffer = vec![0; buffer_size(bytes)];
    let mut warmed = 0usize;
    let mut total_read = 0u64;
    let mut failed = 0usize;

    for track in tracks {
        let path = Path::new(&track.file_path);
        if !path.is_file() {
            continue;
        }

        match prewarm_path_bytes(path, bytes, &mut buffer).await {
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

    tracing::info!(warmed, failed, total_read, bytes, "audio prewarm completed");
    Ok(())
}

pub async fn prewarm_track_path(path: &Path) -> Result<u64> {
    let bytes = audio_prewarm_bytes();
    if bytes == 0 {
        return Ok(0);
    }

    let mut buffer = vec![0; buffer_size(bytes)];
    prewarm_path_bytes(path, bytes, &mut buffer).await
}

fn audio_prewarm_bytes() -> u64 {
    env::var("AUDIO_PREWARM_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_AUDIO_PREWARM_BYTES)
}

fn audio_prewarm_interval() -> Duration {
    Duration::from_secs(
        env::var("AUDIO_PREWARM_INTERVAL_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_AUDIO_PREWARM_INTERVAL_SECONDS),
    )
}

fn buffer_size(bytes: u64) -> usize {
    bytes.clamp(1, PREWARM_BUFFER_BYTES) as usize
}

async fn prewarm_path_bytes(path: &Path, bytes: u64, buffer: &mut [u8]) -> Result<u64> {
    let mut file = File::open(path).await?;
    let mut remaining = bytes;
    let mut total_read = 0u64;

    while remaining > 0 {
        let read_size = remaining.min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..read_size]).await?;
        if read == 0 {
            break;
        }
        remaining -= read as u64;
        total_read += read as u64;
    }

    Ok(total_read)
}
