import { getStreamUrl } from "$lib/services/apiClient";
import type { Track } from "$lib/types";
import type { TrackStoreState } from "$lib/stores/trackStore";
import { parseArtists } from "$lib/utils/formatters";

const NEXT_TRACK_PREFETCH_CHUNKS = 2;

interface ByteRange {
  start: number;
  end: number;
  total: number;
}

export function parseContentRange(value: string | null): ByteRange | null {
  const match = value?.match(/^bytes (\d+)-(\d+)\/(\d+)$/);
  if (!match) return null;

  return {
    start: Number(match[1]),
    end: Number(match[2]),
    total: Number(match[3]),
  };
}

export function getNextTrackForPreload(state: TrackStoreState): Track | null {
  if (state.tracks.length === 0 || state.currentTrackIndex === null) {
    return null;
  }

  if (state.is_shuffle) {
    return state.playHistory[state.historyPosition + 1] ?? null;
  }

  if (state.selectedArtist) {
    const scopedIndexes = state.tracks.reduce<number[]>(
      (indexes, track, index) => {
        if (parseArtists(track.artist).includes(state.selectedArtist!)) {
          indexes.push(index);
        }
        return indexes;
      },
      [],
    );
    const currentScopedIndex = scopedIndexes.indexOf(state.currentTrackIndex);
    if (currentScopedIndex === -1 || scopedIndexes.length === 0) return null;

    return state.tracks[
      scopedIndexes[(currentScopedIndex + 1) % scopedIndexes.length]
    ];
  }

  return state.tracks[(state.currentTrackIndex + 1) % state.tracks.length];
}

export class StreamPreloadService {
  private fetchFn: typeof fetch;
  private nextAbort: AbortController | null = null;
  private nextKey: string | null = null;

  constructor(fetchFn: typeof fetch = fetch) {
    this.fetchFn = fetchFn;
  }

  update(currentTrack: Track | null, nextTrack: Track | null): void {
    const nextKey =
      nextTrack && nextTrack.id !== currentTrack?.id
        ? this.trackKey(nextTrack)
        : null;
    if (nextKey !== this.nextKey) {
      this.nextAbort?.abort();
      this.nextKey = nextKey;

      if (nextTrack && nextKey) {
        const controller = new AbortController();
        this.nextAbort = controller;
        void this.downloadTrack(
          nextTrack,
          controller.signal,
          NEXT_TRACK_PREFETCH_CHUNKS,
        ).catch(this.ignoreAbort);
      }
    }
  }

  destroy(): void {
    this.nextAbort?.abort();
  }

  private async downloadTrack(
    track: Track,
    signal: AbortSignal,
    maxChunks: number,
  ): Promise<void> {
    let start = 0;

    for (let chunk = 0; chunk < maxChunks; chunk += 1) {
      const range = await this.fetchRange(track.id, start, signal);
      if (!range) return;

      if (range.end >= range.total - 1) return;
      start = range.end + 1;
    }
  }

  private async fetchRange(
    trackId: number,
    start: number,
    signal: AbortSignal,
  ): Promise<ByteRange | null> {
    const response = await this.fetchFn(getStreamUrl(trackId), {
      headers: { Range: `bytes=${start}-` },
      signal,
    });
    if (response.status !== 206) return null;

    await response.arrayBuffer();
    return parseContentRange(response.headers.get("content-range"));
  }

  private trackKey(track: Track): string {
    return `${track.id}:${track.updated_at}`;
  }

  private ignoreAbort(error: unknown): void {
    if (error instanceof DOMException && error.name === "AbortError") return;
    console.error("Stream preload failed", error);
  }
}
