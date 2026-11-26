import { describe, expect, it, vi } from "vitest";
import type { Track } from "$lib/types";
import type { TrackState } from "$lib/stores/trackStore";
import {
  firstHlsMediaUrls,
  getNextTrackForPreload,
  HlsPreloadService,
} from "./hlsPreloadService";

function track(id: string, artist = "Artist"): Track {
  return {
    id,
    title: `Track ${id}`,
    artist,
    duration: 120,
    filename: `track-${id}.mp3`,
    updated_at: Number(id),
    has_cover: false,
    cover_small_url: null,
    cover_original_url: null,
    hls_url: `https://music.example/api/v1/tracks/${id}/hls/index.m3u8`,
  };
}

describe("hlsPreloadService", () => {
  it("prefetches the next track playlist and first fMP4 media files", async () => {
    const fetchFn = vi.fn(async (url: string | URL | Request) =>
      String(url).endsWith("index.m3u8")
        ? new Response(
            '#EXTM3U\n#EXT-X-MAP:URI="init.mp4"\n#EXTINF:6,\nsegment-00000.m4s\n',
            { status: 200 },
          )
        : new Response(new Uint8Array(3), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchFn);
    const service = new HlsPreloadService();
    const current = track("1");
    const next = track("2");

    try {
      service.update(current, next);
      await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledTimes(3));
      expect(
        fetchFn.mock.contexts.every((context) => context === globalThis),
      ).toBe(true);
      expect(fetchFn.mock.calls.map(([url]) => String(url))).toEqual([
        "https://music.example/api/v1/tracks/2/hls/index.m3u8",
        "https://music.example/api/v1/tracks/2/hls/init.mp4",
        "https://music.example/api/v1/tracks/2/hls/segment-00000.m4s",
      ]);
    } finally {
      service.destroy();
      vi.unstubAllGlobals();
    }
  });

  it("does not preload the current track as its own next track", async () => {
    const fetchFn = vi.fn(async () => new Response("", { status: 200 }));
    const service = new HlsPreloadService(fetchFn as unknown as typeof fetch);
    const current = track("1");

    service.update(current, current);

    expect(fetchFn).not.toHaveBeenCalled();
    service.destroy();
  });

  it("resolves relative segment URLs from the playlist URL", () => {
    expect(
      firstHlsMediaUrls(
        "https://music.example/api/v1/tracks/2/hls/index.m3u8",
        '#EXTM3U\n#EXT-X-MAP:URI="init.mp4"\n#EXTINF:6,\nsegment-00001.m4s\n',
      ),
    ).toEqual([
      "https://music.example/api/v1/tracks/2/hls/init.mp4",
      "https://music.example/api/v1/tracks/2/hls/segment-00001.m4s",
    ]);
  });

  it("resolves relative playlist URLs against the current page URL", () => {
    expect(
      firstHlsMediaUrls(
        "/api/v1/tracks/2/hls/index.m3u8",
        '#EXTM3U\n#EXT-X-MAP:URI="init.mp4"\n#EXTINF:6,\nsegment-00001.m4s\n',
      ),
    ).toEqual([
      `${window.location.origin}/api/v1/tracks/2/hls/init.mp4`,
      `${window.location.origin}/api/v1/tracks/2/hls/segment-00001.m4s`,
    ]);
  });

  it("derives deterministic next track for preload", () => {
    const tracks = [track("1"), track("2", "Other"), track("3")];
    const baseState: TrackState = {
      tracks,
      tracksLoading: false,
      currentTrackId: tracks[0].id,
      playHistoryIds: [tracks[0].id],
      historyPosition: 0,
      is_shuffle: false,
      selectedArtist: null,
      selectedCategory: null,
      playRequestId: 0,
    };

    expect(getNextTrackForPreload(baseState)).toBe(tracks[1]);
    expect(
      getNextTrackForPreload({ ...baseState, currentTrackId: tracks[2].id }),
    ).toBe(tracks[0]);
    expect(
      getNextTrackForPreload({ ...baseState, selectedArtist: "Artist" }),
    ).toBe(tracks[2]);
    expect(
      getNextTrackForPreload({
        ...baseState,
        is_shuffle: true,
        playHistoryIds: [tracks[0].id, tracks[2].id],
      }),
    ).toBe(tracks[2]);
    expect(
      getNextTrackForPreload({ ...baseState, is_shuffle: true }),
    ).toBeNull();
    tracks[0].tags = tracks[2].tags = [
      { name: "cover", display_name: "Cover" },
    ];
    expect(
      getNextTrackForPreload({ ...baseState, selectedCategory: "cover" }),
    ).toBe(tracks[2]);
    expect(
      getNextTrackForPreload({
        ...baseState,
        selectedCategory: "cover",
        is_shuffle: true,
        playHistoryIds: tracks.map((track) => track.id),
      }),
    ).toBe(tracks[2]);
  });
});
