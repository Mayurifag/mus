import { afterEach, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { handleMusEvent } from "./eventHandlerService";
import { trackStore } from "$lib/stores/trackStore";
import { downloadStore } from "$lib/stores/downloadStore";
import type { Track } from "$lib/types";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  trackStore.reset();
  downloadStore.reset();
});

it("keeps the loaded library when an event-triggered refresh fails", async () => {
  const track: Track = {
    id: "existing",
    title: "Song",
    artist: "Artist",
    duration: 120,
    filename: "song.mp3",
    has_cover: false,
    updated_at: 1,
    cover_small_url: null,
    cover_original_url: null,
    hls_url: "/api/v1/tracks/existing/hls/1/index.m3u8",
  };
  trackStore.setTracks([track]);
  const log = vi.spyOn(console, "error").mockImplementation(() => {});
  const response = new Response(
    JSON.stringify({ detail: "Service unavailable" }),
    { status: 503 },
  );
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response));
  handleMusEvent({
    action_key: "download_completed",
    message_to_show: null,
    message_level: null,
    action_payload: null,
  });
  await vi.waitFor(() => expect(log).toHaveBeenCalled());
  expect(get(trackStore).tracks).toEqual([track]);
  expect(get(trackStore).currentTrack).toEqual(track);
});
