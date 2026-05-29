import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import type { Track } from "$lib/types";
import type { TrackStoreState } from "$lib/stores/trackStore";
import {
  getNextTrackForPreload,
  parseContentRange,
  StreamPreloadService,
} from "./streamPreloadService";

function track(id: number, artist = "Artist"): Track {
  return {
    id,
    title: `Track ${id}`,
    artist,
    duration: 120,
    filename: `track-${id}.mp3`,
    updated_at: id,
    has_cover: false,
    cover_small_url: null,
    cover_original_url: null,
  };
}

function responseForRange(start: number, total = 8, chunkSize = 3): Response {
  const end = Math.min(start + chunkSize - 1, total - 1);
  return new Response(new Uint8Array(end - start + 1), {
    status: 206,
    headers: { "Content-Range": `bytes ${start}-${end}/${total}` },
  });
}

function rangeHeader(init: RequestInit | undefined): string | null {
  return new Headers(init?.headers).get("range");
}

describe("streamPreloadService", () => {
  it("parses content-range headers", () => {
    expect(parseContentRange("bytes 3-5/8")).toEqual({
      start: 3,
      end: 5,
      total: 8,
    });
    expect(parseContentRange(null)).toBeNull();
    expect(parseContentRange("invalid")).toBeNull();
  });

  it("fully downloads current track and prefetches two next track chunks", async () => {
    const fetchFn = vi.fn(
      async (_url: string | URL | Request, init?: RequestInit) => {
        const start = Number(
          rangeHeader(init)?.match(/^bytes=(\d+)-$/)?.[1] ?? 0,
        );
        return responseForRange(start);
      },
    );
    const service = new StreamPreloadService(
      fetchFn as unknown as typeof fetch,
    );
    const current = track(1);
    const next = track(2);

    service.update(current, next);

    await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledTimes(5));
    const currentRanges = fetchFn.mock.calls
      .filter(([url]) => String(url).includes("/tracks/1/stream"))
      .map(([, init]) => rangeHeader(init));
    const nextRanges = fetchFn.mock.calls
      .filter(([url]) => String(url).includes("/tracks/2/stream"))
      .map(([, init]) => rangeHeader(init));

    expect(currentRanges).toEqual(["bytes=0-", "bytes=3-", "bytes=6-"]);
    expect(nextRanges).toEqual(["bytes=0-", "bytes=3-"]);
    expect(get(service.downloadedRangesStore)).toEqual([
      { start: 0, end: 120 },
    ]);

    service.destroy();
  });

  it("derives deterministic next track for preload", () => {
    const tracks = [track(1), track(2, "Other"), track(3)];
    const baseState: TrackStoreState = {
      tracks,
      currentTrackIndex: 0,
      playHistory: [tracks[0]],
      historyPosition: 0,
      currentTrack: tracks[0],
      is_shuffle: false,
      selectedArtist: null,
      playRequestId: 0,
    };

    expect(getNextTrackForPreload(baseState)).toBe(tracks[1]);
    expect(
      getNextTrackForPreload({ ...baseState, selectedArtist: "Artist" }),
    ).toBe(tracks[2]);
    expect(
      getNextTrackForPreload({
        ...baseState,
        is_shuffle: true,
        playHistory: [tracks[0], tracks[2]],
      }),
    ).toBe(tracks[2]);
  });
});
