import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import { get } from "svelte/store";
import { trackStore } from "$lib/stores/trackStore";
import { permissionsStore } from "$lib/stores/permissionsStore";
import { audioServiceStore } from "$lib/stores/audioServiceStore";
import { downloadStore } from "$lib/stores/downloadStore";
import Layout from "./+layout.svelte";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function json(data: unknown) {
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
}

describe("initial app loading", () => {
  afterEach(() => {
    trackStore.reset();
    downloadStore.reset();
    permissionsStore.set({ can_write_music_files: false });
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("shows tracks and restores playback without waiting for permissions", async () => {
    const tracks = deferred<Response>();
    const player = deferred<Response>();
    const permissions = deferred<Response>();
    const track = (id: string) => ({
      id,
      title: `Song ${id}`,
      artist: "Artist",
      duration: 180,
      filename: `song-${id}.mp3`,
      has_cover: false,
      cover_small_url: null,
      cover_original_url: null,
      hls_url: `/api/v1/tracks/${id}/hls/1/index.m3u8`,
      updated_at: 1,
    });
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith("/errors/tracks")) return Promise.resolve(json([]));
        if (url.endsWith("/tracks")) return tracks.promise;
        if (url.endsWith("/player/state")) return player.promise;
        if (url.endsWith("/system/permissions")) return permissions.promise;
        if (url.endsWith("/system/info")) {
          return Promise.resolve(
            json({
              app_date: "unknown",
              commit_sha: null,
              music_dir: { warning: null },
              yt_dlp_version: null,
            }),
          );
        }
        return Promise.resolve(new Response(null, { status: 404 }));
      }),
    );
    vi.stubGlobal(
      "EventSource",
      class {
        close() {}
      },
    );
    vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => {});
    vi.spyOn(HTMLMediaElement.prototype, "load").mockImplementation(() => {});

    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    const tracksResponse = json([track("1"), track("2")]);
    const readTracks = vi.spyOn(tracksResponse, "json");
    tracks.resolve(tracksResponse);
    await waitFor(() => expect(readTracks).toHaveBeenCalled());
    expect(get(trackStore).tracksLoading).toBe(true);
    expect(get(trackStore).currentTrack).toBeNull();

    player.resolve(
      json({
        current_track_id: "2",
        progress_seconds: 32,
        volume_level: 0.7,
        is_muted: false,
        is_playing: true,
        is_shuffle: false,
        is_repeat: false,
      }),
    );

    await waitFor(() => expect(get(trackStore).tracksLoading).toBe(false));
    expect(get(trackStore).tracks).toHaveLength(2);
    expect(get(trackStore).currentTrack?.id).toBe("2");
    expect(get(audioServiceStore)?.currentTime).toBe(32);
    expect(get(audioServiceStore)?.isPlaying).toBe(false);
    expect(get(permissionsStore).can_write_music_files).toBe(false);

    permissions.resolve(json({ can_write_music_files: true }));
    await waitFor(() =>
      expect(get(permissionsStore).can_write_music_files).toBe(true),
    );
    expect(get(trackStore).currentTrack?.id).toBe("2");
    unmount();
  });

  it("does not confirm a download before the event connection opens", async () => {
    const player = deferred<Response>();
    const connections: Array<{ onopen?: () => void }> = [];
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/player/state")) return player.promise;
      if (url.endsWith("/system/permissions"))
        return Promise.resolve(json({ can_write_music_files: true }));
      if (url.endsWith("/system/info"))
        return Promise.resolve(
          json({
            app_date: "unknown",
            commit_sha: null,
            music_dir: { warning: null },
            yt_dlp_version: null,
          }),
        );
      if (url.endsWith("/downloads/metadata"))
        return Promise.resolve(
          json({
            title: "Song",
            artist: "Artist",
            thumbnail_url: null,
            duration: null,
            tags: [],
          }),
        );
      if (url.endsWith("/downloads/confirm"))
        return Promise.resolve(json({ status: "accepted" }));
      return Promise.resolve(json([]));
    });
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal(
      "EventSource",
      class {
        onopen?: () => void;
        constructor() {
          connections.push(this);
        }
        close() {}
      },
    );
    vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => {});
    vi.spyOn(HTMLMediaElement.prototype, "load").mockImplementation(() => {});

    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    const input = await screen.findByPlaceholderText(
      "Enter YouTube URL or other supported link",
    );
    await fireEvent.input(input, {
      target: { value: "https://example.invalid/test" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Download" }));
    const save = await screen.findByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    expect(connections).toHaveLength(0);

    player.resolve(
      json({
        current_track_id: null,
        progress_seconds: 0,
        volume_level: 1,
        is_muted: false,
        is_playing: false,
        is_shuffle: false,
        is_repeat: false,
      }),
    );
    await waitFor(() => expect(connections).toHaveLength(1));
    expect(save).toBeDisabled();
    expect(
      fetchMock.mock.calls.some(([url]) =>
        String(url).endsWith("/downloads/confirm"),
      ),
    ).toBe(false);
    connections[0].onopen?.();
    await waitFor(() => expect(save).toBeEnabled());
    await fireEvent.click(save);
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(([url]) =>
          String(url).endsWith("/downloads/confirm"),
        ),
      ).toBe(true),
    );
    unmount();
  });

  it("uses one download editor while both sidebars are mounted", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith("/system/permissions"))
          return Promise.resolve(json({ can_write_music_files: true }));
        if (url.endsWith("/system/info"))
          return Promise.resolve(
            json({
              app_date: "unknown",
              commit_sha: null,
              music_dir: { warning: null },
              yt_dlp_version: null,
            }),
          );
        if (url.endsWith("/player/state"))
          return Promise.resolve(
            json({
              current_track_id: null,
              progress_seconds: 0,
              volume_level: 1,
              is_muted: false,
              is_playing: false,
              is_shuffle: false,
              is_repeat: false,
            }),
          );
        return Promise.resolve(json([]));
      }),
    );
    vi.stubGlobal(
      "EventSource",
      class {
        close() {}
      },
    );
    vi.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => {});
    vi.spyOn(HTMLMediaElement.prototype, "load").mockImplementation(() => {});

    const children = createRawSnippet(() => ({ render: () => "<div></div>" }));
    const { unmount } = render(Layout, { props: { children } });
    await waitFor(() => expect(get(trackStore).tracksLoading).toBe(false));
    document.body.dispatchEvent(new CustomEvent("toggle-sheet"));
    await waitFor(() =>
      expect(
        screen.getAllByPlaceholderText(
          "Enter YouTube URL or other supported link",
        ),
      ).toHaveLength(2),
    );
    downloadStore.setFetchingMetadata("https://example.invalid/test");
    downloadStore.setAwaitingReview({
      title: "Song",
      artist: "Artist",
      thumbnailUrl: null,
      duration: null,
    });
    expect(await screen.findByLabelText("Title")).toHaveValue("Song");
    expect(screen.getAllByLabelText("Title")).toHaveLength(1);
    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() =>
      expect(screen.queryByLabelText("Title")).not.toBeInTheDocument(),
    );
    expect(get(downloadStore).state).toBe("idle");
    unmount();
  });
});
