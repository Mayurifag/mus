import { getStreamUrl } from "$lib/services/apiClient";
import type { Track } from "$lib/types";
import type { TrackStoreState } from "$lib/stores/trackStore";
import { audioStreamRangeHeader } from "$lib/utils/audioStreamRange";
import { parseArtists } from "$lib/utils/formatters";

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
        void this.downloadTrack(nextTrack, controller.signal).catch(
          this.ignoreAbort,
        );
      }
    }
  }

  destroy(): void {
    this.nextAbort?.abort();
  }

  private async downloadTrack(
    track: Track,
    signal: AbortSignal,
  ): Promise<void> {
    const response = await this.fetchFn(getStreamUrl(track.id), {
      headers: {
        Range: audioStreamRangeHeader(0),
      },
      signal,
    });
    if (response.status !== 206) return;

    await response.arrayBuffer();
  }

  private trackKey(track: Track): string {
    return `${track.id}:${track.updated_at}`;
  }

  private ignoreAbort(error: unknown): void {
    if (error instanceof DOMException && error.name === "AbortError") return;
    console.error("Stream preload failed", error);
  }
}
