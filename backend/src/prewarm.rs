use std::{
    collections::{HashMap, VecDeque},
    env,
    path::Path,
    sync::Arc,
};

use anyhow::Result;
use tokio::{fs::File, io::AsyncReadExt};

use crate::{db, models::Track, state::AppState, util::file_signature};

const DEFAULT_AUDIO_PREWARM_BYTES: u64 = 512 * 1024;
const DEFAULT_AUDIO_MEMORY_CACHE_BYTES: usize = 256 * 1024 * 1024;
const PREWARM_BUFFER_BYTES: u64 = 64 * 1024;

#[derive(Clone)]
pub struct CachedAudioStart {
    pub file_len: u64,
    pub bytes: Arc<[u8]>,
}

struct AudioMemoryCacheEntry {
    signature: String,
    file_len: u64,
    bytes: Arc<[u8]>,
    last_used: u64,
}

pub struct AudioMemoryCache {
    capacity_bytes: usize,
    used_bytes: usize,
    next_use: u64,
    entries: HashMap<i64, AudioMemoryCacheEntry>,
    order: VecDeque<(i64, u64)>,
}

impl AudioMemoryCache {
    pub fn from_env() -> Self {
        Self::new(audio_memory_cache_bytes())
    }

    fn new(capacity_bytes: usize) -> Self {
        Self {
            capacity_bytes,
            used_bytes: 0,
            next_use: 0,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, track_id: i64, signature: &str) -> Option<CachedAudioStart> {
        let entry = self.entries.get_mut(&track_id)?;
        if entry.signature != signature {
            self.used_bytes = self.used_bytes.saturating_sub(entry.bytes.len());
            self.entries.remove(&track_id);
            return None;
        }

        self.next_use = self.next_use.wrapping_add(1);
        entry.last_used = self.next_use;
        self.order.push_back((track_id, entry.last_used));
        Some(CachedAudioStart {
            file_len: entry.file_len,
            bytes: entry.bytes.clone(),
        })
    }

    pub fn insert(
        &mut self,
        track_id: i64,
        signature: String,
        file_len: u64,
        bytes: Vec<u8>,
    ) -> Option<CachedAudioStart> {
        if self.capacity_bytes == 0 || bytes.is_empty() || bytes.len() > self.capacity_bytes {
            return None;
        }

        if let Some(existing) = self.entries.remove(&track_id) {
            self.used_bytes = self.used_bytes.saturating_sub(existing.bytes.len());
        }

        self.next_use = self.next_use.wrapping_add(1);
        let bytes = Arc::<[u8]>::from(bytes);
        self.used_bytes += bytes.len();
        self.order.push_back((track_id, self.next_use));
        self.entries.insert(
            track_id,
            AudioMemoryCacheEntry {
                signature,
                file_len,
                bytes: bytes.clone(),
                last_used: self.next_use,
            },
        );
        self.evict_if_needed();

        self.entries
            .contains_key(&track_id)
            .then_some(CachedAudioStart { file_len, bytes })
    }

    pub fn prewarm_track_limit(&self, bytes_per_track: u64) -> usize {
        let Ok(bytes_per_track) = usize::try_from(bytes_per_track) else {
            return 0;
        };
        if self.capacity_bytes == 0 || bytes_per_track == 0 {
            return 0;
        }

        self.capacity_bytes / bytes_per_track
    }

    pub fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    #[cfg(test)]
    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    fn evict_if_needed(&mut self) {
        while self.used_bytes > self.capacity_bytes {
            let Some((track_id, last_used)) = self.order.pop_front() else {
                break;
            };
            let should_remove = self
                .entries
                .get(&track_id)
                .is_some_and(|entry| entry.last_used == last_used);
            if should_remove {
                if let Some(entry) = self.entries.remove(&track_id) {
                    self.used_bytes = self.used_bytes.saturating_sub(entry.bytes.len());
                }
            }
        }
    }
}

pub async fn prewarm_music_dir(state: AppState) -> Result<()> {
    let bytes = audio_prewarm_bytes();
    let limit = state.audio_cache.lock().await.prewarm_track_limit(bytes);
    if limit == 0 {
        return Ok(());
    }

    let tracks = db::list_tracks(&state)?;
    let mut warmed = 0usize;
    let mut total_read = 0u64;
    let mut failed = 0usize;

    for track in tracks.into_iter().take(limit) {
        match cache_track_start(&state, &track).await {
            Ok(Some(cached)) => {
                warmed += 1;
                total_read += cached.bytes.len() as u64;
            }
            Ok(None) => {}
            Err(error) => {
                failed += 1;
                tracing::warn!(path = %track.file_path, "failed to prewarm audio file: {error}");
            }
        }
    }

    tracing::info!(warmed, failed, total_read, bytes, "audio prewarm completed");
    Ok(())
}

pub async fn cache_track_start(
    state: &AppState,
    track: &Track,
) -> Result<Option<CachedAudioStart>> {
    let mut bytes = audio_prewarm_bytes();
    if bytes == 0 {
        return Ok(None);
    }

    let path = Path::new(&track.file_path);
    let meta = tokio::fs::metadata(path).await?;
    if !meta.is_file() {
        return Ok(None);
    }
    let capacity = state.audio_cache.lock().await.capacity_bytes();
    if capacity == 0 {
        return Ok(None);
    }
    bytes = bytes.min(meta.len()).min(capacity as u64);
    if bytes == 0 {
        return Ok(None);
    }

    let signature = file_signature(&meta);
    if let Some(cached) = state.audio_cache.lock().await.get(track.id, &signature) {
        return Ok(Some(cached));
    }

    let data = read_path_start(path, bytes).await?;
    Ok(state
        .audio_cache
        .lock()
        .await
        .insert(track.id, signature, meta.len(), data))
}

pub async fn cached_track_start(
    state: &AppState,
    track_id: i64,
    signature: &str,
) -> Option<CachedAudioStart> {
    state.audio_cache.lock().await.get(track_id, signature)
}

fn audio_prewarm_bytes() -> u64 {
    env::var("AUDIO_PREWARM_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_AUDIO_PREWARM_BYTES)
}

fn audio_memory_cache_bytes() -> usize {
    env::var("AUDIO_MEMORY_CACHE_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_AUDIO_MEMORY_CACHE_BYTES)
}

fn buffer_size(bytes: u64) -> usize {
    bytes.clamp(1, PREWARM_BUFFER_BYTES) as usize
}

async fn read_path_start(path: &Path, bytes: u64) -> Result<Vec<u8>> {
    let mut file = File::open(path).await?;
    let mut remaining = bytes;
    let mut buffer = vec![0; buffer_size(bytes)];
    let mut data = Vec::with_capacity(bytes.min(usize::MAX as u64) as usize);

    while remaining > 0 {
        let read_size = remaining.min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..read_size]).await?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..read]);
        remaining -= read as u64;
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::AudioMemoryCache;

    #[test]
    fn evicts_least_recently_used_audio_starts() {
        let mut cache = AudioMemoryCache::new(4);

        cache.insert(1, "a".into(), 10, vec![1, 2]);
        cache.insert(2, "b".into(), 10, vec![3, 4]);
        cache.get(1, "a");
        cache.insert(3, "c".into(), 10, vec![5, 6]);

        assert!(cache.get(1, "a").is_some());
        assert!(cache.get(2, "b").is_none());
        assert!(cache.get(3, "c").is_some());
        assert_eq!(cache.used_bytes(), 4);
    }
}
