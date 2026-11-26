import type { Track } from "$lib/types";
import type { TrackState } from "$lib/stores/trackStore";
import { selectedTrackIndexes } from "$lib/stores/filteredTrackNavigation";
import { trackIndex } from "$lib/stores/trackNavigation";

export function getNextTrackForPreload(state: TrackState): Track | null {
  if (state.tracks.length === 0 || state.currentTrackId === null) {
    return null;
  }

  const { tracks, currentTrackId, playHistoryIds, historyPosition } = state;
  const filtered = state.selectedArtist || state.selectedCategory;
  if (state.is_shuffle) {
    if (historyPosition + 1 >= playHistoryIds.length) return null;
    const indexes = filtered ? new Set(selectedTrackIndexes(state)) : null;
    for (let i = historyPosition + 1; i < playHistoryIds.length; i++) {
      const index = trackIndex(tracks, playHistoryIds[i]);
      if (index !== -1 && (!indexes || indexes.has(index)))
        return tracks[index];
    }
    return null;
  }
  const current = trackIndex(tracks, currentTrackId);
  if (!filtered) return tracks[(current + 1) % tracks.length];
  const indexes = selectedTrackIndexes(state);
  if (indexes.length === 0) return null;
  return tracks[indexes[(indexes.indexOf(current) + 1) % indexes.length]];
}

export class HlsPreloadService {
  private fetchFn: typeof fetch;
  private nextAbort: AbortController | null = null;
  private nextKey: string | null = null;

  constructor(fetchFn: typeof fetch = globalThis.fetch.bind(globalThis)) {
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
