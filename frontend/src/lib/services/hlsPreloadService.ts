import type { Track } from "$lib/types";
import type { TrackStoreState } from "$lib/stores/trackStore";
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

export class HlsPreloadService {
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
        void this.preloadTrack(nextTrack, controller.signal).catch(
          this.ignoreAbort,
        );
      }
    }
  }

  destroy(): void {
    this.nextAbort?.abort();
  }

  private async preloadTrack(track: Track, signal: AbortSignal): Promise<void> {
    const playlistResponse = await this.fetchFn(track.hls_url, { signal });
    if (!playlistResponse.ok) return;

    const mediaUrls = firstHlsMediaUrls(
      track.hls_url,
      await playlistResponse.text(),
    );

    for (const mediaUrl of mediaUrls) {
      const mediaResponse = await this.fetchFn(mediaUrl, { signal });
      if (mediaResponse.ok) {
        await mediaResponse.arrayBuffer();
      }
    }
  }

  private trackKey(track: Track): string {
    return `${track.id}:${track.hls_url}`;
  }

  private ignoreAbort(error: unknown): void {
    if (error instanceof DOMException && error.name === "AbortError") return;
    console.error("HLS preload failed", error);
  }
}

export function firstHlsMediaUrls(
  playlistUrl: string,
  playlist: string,
): string[] {
  const resolvedPlaylistUrl = new URL(
    playlistUrl,
    globalThis.location?.href,
  ).toString();
  const lines = playlist.split("\n").map((value) => value.trim());
  const urls: string[] = [];
  const init = lines
    .find((value) => value.startsWith("#EXT-X-MAP:"))
    ?.match(/URI="([^"]+)"/)?.[1];
  if (init) {
    urls.push(new URL(init, resolvedPlaylistUrl).toString());
  }

  const segment = lines.find((value) => value !== "" && !value.startsWith("#"));
  if (segment) {
    urls.push(new URL(segment, resolvedPlaylistUrl).toString());
  }

  return urls;
}
