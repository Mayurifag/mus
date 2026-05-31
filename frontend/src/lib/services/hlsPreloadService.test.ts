import { describe, expect, it, vi } from "vitest";
import type { Track } from "$lib/types";
import type { TrackStoreState } from "$lib/stores/trackStore";
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
    hls_url: `https://music.example/api/v1/tracks/${id}/hls/${id}/index.m3u8`,
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
    const service = new HlsPreloadService(fetchFn as unknown as typeof fetch);
    const current = track("1");
    const next = track("2");

    service.update(current, next);

    await vi.waitFor(() => expect(fetchFn).toHaveBeenCalledTimes(3));
    expect(fetchFn.mock.calls.map(([url]) => String(url))).toEqual([
      "https://music.example/api/v1/tracks/2/hls/2/index.m3u8",
      "https://music.example/api/v1/tracks/2/hls/2/init.mp4",
      "https://music.example/api/v1/tracks/2/hls/2/segment-00000.m4s",
    ]);

    service.destroy();
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
        "https://music.example/api/v1/tracks/2/hls/2/index.m3u8",
        '#EXTM3U\n#EXT-X-MAP:URI="init.mp4"\n#EXTINF:6,\nsegment-00001.m4s\n',
      ),
    ).toEqual([
      "https://music.example/api/v1/tracks/2/hls/2/init.mp4",
      "https://music.example/api/v1/tracks/2/hls/2/segment-00001.m4s",
    ]);
  });

  it("resolves relative playlist URLs against the current page URL", () => {
    expect(
      firstHlsMediaUrls(
        "/api/v1/tracks/2/hls/2/index.m3u8",
        '#EXTM3U\n#EXT-X-MAP:URI="init.mp4"\n#EXTINF:6,\nsegment-00001.m4s\n',
      ),
    ).toEqual([
      `${window.location.origin}/api/v1/tracks/2/hls/2/init.mp4`,
      `${window.location.origin}/api/v1/tracks/2/hls/2/segment-00001.m4s`,
    ]);
  });

  it("derives deterministic next track for preload", () => {
    const tracks = [track("1"), track("2", "Other"), track("3")];
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
