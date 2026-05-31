import { describe, expect, it, vi } from "vitest";
import type { Track } from "$lib/types";
import type { TrackStoreState } from "$lib/stores/trackStore";
import {
  getNextTrackForPreload,
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

function rangeHeader(init: RequestInit | undefined): string | null {
  return new Headers(init?.headers).get("range");
}

describe("streamPreloadService", () => {
  it("prefetches one next track chunk without downloading current track", async () => {
    const fetchFn = vi.fn(
      async (url: string | URL | Request, init?: RequestInit) =>
        new Response(
          new Uint8Array(
            String(url).includes("/tracks/2/stream") && rangeHeader(init)
              ? 3
              : 0,
          ),
          { status: 206 },
        ),
    );
    const service = new StreamPreloadService(
      fetchFn as unknown as typeof fetch,
    );
    const current = track(1);
    const next = track(2);

    service.update(current, next);

    await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledTimes(1));
    const currentRanges = fetchFn.mock.calls
      .filter(([url]) => String(url).includes("/tracks/1/stream"))
      .map(([, init]) => rangeHeader(init));
    const nextRanges = fetchFn.mock.calls
      .filter(([url]) => String(url).includes("/tracks/2/stream"))
      .map(([, init]) => rangeHeader(init));

    expect(currentRanges).toEqual([]);
    expect(nextRanges).toEqual(["bytes=0-262143"]);

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
