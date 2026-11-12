import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, waitFor } from "@testing-library/svelte";
import { createRawSnippet, tick } from "svelte";
import { get } from "svelte/store";
import Layout from "./+layout.svelte";
import { trackStore } from "$lib/stores/trackStore";
import { audioServiceStore } from "$lib/stores/audioServiceStore";
import { handleMusEvent } from "$lib/services/eventHandlerService";
import type { Track } from "$lib/types";

const hls = vi.hoisted(() => ({
  instances: [] as Array<{ destroy: ReturnType<typeof vi.fn> }>,
}));
vi.mock("hls.js/light", () => ({
  default: class {
    static Events = { ERROR: "hlsError" };
    static isSupported = () => true;
    loadSource = vi.fn();
    attachMedia = vi.fn((audio: HTMLAudioElement) => {
      audio.src = "blob:active-audio";
    });
    on = vi.fn();
    destroy = vi.fn();
    constructor() {
      hls.instances.push(this);
    }
  },
}));

const track: Track = {
  id: "last",
  title: "Last track",
  artist: "Artist",
  filename: "last.mp3",
  duration: 180,
  updated_at: 1,
  has_cover: false,
  cover_original_url: null,
  cover_small_url: null,
  hls_url: "/api/v1/tracks/last/hls/1/index.m3u8",
};
const session = {
  metadata: null as MediaMetadata | null,
  playbackState: "none",
  setActionHandler: vi.fn(),
};
let originalSession: PropertyDescriptor | undefined;
let originalTitle: string;
let savedTrackId: string | null;

beforeEach(() => {
  savedTrackId = track.id;
  originalTitle = document.title;
  document.title = "mus";
  originalSession = Object.getOwnPropertyDescriptor(navigator, "mediaSession");
  Object.defineProperty(navigator, "mediaSession", {
    configurable: true,
    value: session,
  });
  vi.stubGlobal(
    "MediaMetadata",
    class {
      constructor(data: MediaMetadataInit) {
        Object.assign(this, data);
      }
    },
  );
  session.metadata = null;
  session.playbackState = "none";
  session.setActionHandler.mockClear();
  hls.instances.length = 0;
  trackStore.reset();
  const json = (data: unknown) => new Response(JSON.stringify(data));
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/tracks"))
        return Promise.resolve(
          json(url.endsWith("/errors/tracks") ? [] : [track]),
        );
      if (url.endsWith("/player/state"))
        return Promise.resolve(
          json({
            current_track_id: savedTrackId,
            progress_seconds: 32,
            volume_level: 1,
            is_muted: false,
            is_playing: false,
            is_shuffle: false,
            is_repeat: false,
          }),
        );
      if (url.endsWith("/system/permissions"))
        return Promise.resolve(json({ can_write_music_files: false }));
      return Promise.resolve(
        json({
          app_date: "unknown",
          commit_sha: null,
          music_dir: { warning: null },
          yt_dlp_version: null,
        }),
      );
    }),
  );
  vi.stubGlobal(
    "EventSource",
    class {
      close() {}
    },
  );
  vi.spyOn(HTMLMediaElement.prototype, "play").mockImplementation(function (
    this: HTMLMediaElement,
  ) {
    this.dispatchEvent(new Event("play"));
    this.dispatchEvent(new Event("playing"));
    return Promise.resolve();
  });
  vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(function (
    this: HTMLMediaElement,
  ) {
    this.dispatchEvent(new Event("pause"));
  });
  vi.spyOn(HTMLMediaElement.prototype, "load").mockImplementation(() => {});
});

afterEach(() => {
  trackStore.reset();
  if (originalSession)
    Object.defineProperty(navigator, "mediaSession", originalSession);
  else Reflect.deleteProperty(navigator, "mediaSession");
  document.title = originalTitle;
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("selection cleanup through the layout", () => {
  it("initializes the fallback selection when the saved track no longer exists", async () => {
    savedTrackId = "missing";
    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    try {
      await waitFor(() => expect(document.title).toBe("Artist - Last track"));
      const service = get(audioServiceStore)!;
      expect(service.isPlaying).toBe(false);
      expect(service.currentTime).toBe(0);
      service.play();
      expect(service.isPlaying).toBe(true);
      expect(hls.instances).toHaveLength(1);
    } finally {
      unmount();
    }
  });

  it("refreshes edited playback metadata without replacing the active stream", async () => {
    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    try {
      await waitFor(() => expect(get(audioServiceStore)?.currentTime).toBe(32));
      const service = get(audioServiceStore)!;
      const audio = document.querySelector("audio")!;
      service.play();
      const source = audio.getAttribute("src");
      trackStore.updateTrack({ ...track, title: "Updated title" });
      await tick();

      expect(document.title).toBe("Artist - Updated title");
      expect(session.metadata?.title).toBe("Updated title");
      expect(service.isPlaying).toBe(true);
      expect(audio.getAttribute("src")).toBe(source);
      expect(hls.instances).toHaveLength(1);
      expect(hls.instances[0].destroy).not.toHaveBeenCalled();
    } finally {
      unmount();
    }
  });

  it("keeps playing when a scanner update replaces the track ID", async () => {
    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    try {
      await waitFor(() => expect(get(audioServiceStore)?.currentTime).toBe(32));
      const service = get(audioServiceStore)!;
      service.play();
      const updated = {
        ...track,
        id: "updated",
        hls_url: "/api/v1/tracks/updated/hls/2/index.m3u8",
      };
      vi.stubGlobal(
        "fetch",
        vi.fn().mockResolvedValue(new Response(JSON.stringify([updated]))),
      );

      handleMusEvent({
        action_key: "track_updated",
        action_payload: { ...updated, previous_id: track.id },
        message_to_show: null,
        message_level: "info",
      });

      await waitFor(() =>
        expect(get(trackStore).currentTrack?.id).toBe(updated.id),
      );
      await tick();
      expect(service.isPlaying).toBe(true);
      expect(get(trackStore).playHistory[0].id).toBe(updated.id);
      expect(hls.instances).toHaveLength(2);
    } finally {
      unmount();
    }
  });

  it("stops the last playing track and releases its HLS and media-session resources", async () => {
    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    try {
      await waitFor(() =>
        expect(get(trackStore).currentTrack?.id).toBe(track.id),
      );
      const service = get(audioServiceStore)!;
      const audio = document.querySelector("audio")!;
      service.play();
      expect(service.isPlaying).toBe(true);
      expect(hls.instances).toHaveLength(1);
      expect(session.metadata).not.toBeNull();
      vi.stubGlobal("crypto", {});
      handleMusEvent({
        action_key: "track_deleted",
        action_payload: { id: track.id },
        message_to_show: "Deleted track",
        message_level: "info",
      });
      await tick();
      expect(get(trackStore).currentTrack).toBeNull();
      expect(service.isPlaying).toBe(false);
      expect(hls.instances[0].destroy).toHaveBeenCalledOnce();
      expect(audio.getAttribute("src")).toBeNull();
      expect(service.currentTime).toBe(0);
      expect(service.duration).toBe(0);
      expect(service.currentBufferedRanges).toEqual([]);
      expect(session.metadata).toBeNull();
      expect(session.playbackState).toBe("none");
      expect(session.setActionHandler).toHaveBeenCalledWith("play", null);
      expect(document.title).toBe("mus");

      trackStore.addTrack(track);
      trackStore.playTrack(0);
      await tick();
      expect(hls.instances).toHaveLength(2);
      expect(service.isPlaying).toBe(true);
    } finally {
      unmount();
    }
  });

  it("clears a deleted paused selection before a late metadata event can load it", async () => {
    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    try {
      await waitFor(() => expect(get(audioServiceStore)?.currentTime).toBe(32));
      expect(hls.instances).toHaveLength(0);
      trackStore.deleteTrack(track.id);
      await tick();
      document
        .querySelector("audio")!
        .dispatchEvent(new Event("loadedmetadata"));
      expect(hls.instances).toHaveLength(0);
      expect(get(audioServiceStore)?.currentTime).toBe(0);
    } finally {
      unmount();
    }
  });
});
