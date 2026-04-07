import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { downloadStore } from "./downloadStore";

describe("downloadStore", () => {
  beforeEach(() => {
    downloadStore.reset();
  });

  it("should have initial state", () => {
    const state = get(downloadStore);
    expect(state).toEqual({
      state: "idle",
      error: null,
      url: null,
      title: null,
      artist: null,
      thumbnailUrl: null,
      duration: null,
      progress: null,
    });
  });

  it("should start download", () => {
    downloadStore.startDownload();
    const state = get(downloadStore);
    expect(state.state).toBe("downloading");
    expect(state.error).toBeNull();
  });

  it("should set completed state", () => {
    downloadStore.startDownload();
    downloadStore.setCompleted();
    const state = get(downloadStore);
    expect(state.state).toBe("completed");
    expect(state.error).toBeNull();
  });

  it("should set failed state", () => {
    const errorMessage = "Download failed";
    downloadStore.setFailed(errorMessage);
    const state = get(downloadStore);
    expect(state.state).toBe("failed");
    expect(state.error).toBe(errorMessage);
  });

  it("should reset to initial state", () => {
    downloadStore.startDownload();
    downloadStore.setFailed("Some error");
    downloadStore.reset();
    const state = get(downloadStore);
    expect(state).toEqual({
      state: "idle",
      error: null,
      url: null,
      title: null,
      artist: null,
      thumbnailUrl: null,
      duration: null,
      progress: null,
    });
  });

  it("should clear error when starting new download", () => {
    downloadStore.setFailed("Previous error");
    downloadStore.startDownload();
    const state = get(downloadStore);
    expect(state.state).toBe("downloading");
    expect(state.error).toBeNull();
  });

  it("should set fetching_metadata state with url", () => {
    downloadStore.setFetchingMetadata("https://example.com/video");
    const state = get(downloadStore);
    expect(state.state).toBe("fetching_metadata");
    expect(state.url).toBe("https://example.com/video");
    expect(state.error).toBeNull();
  });

  it("should set awaiting_review state with metadata fields", () => {
    downloadStore.setFetchingMetadata("https://example.com/video");
    downloadStore.setAwaitingReview({
      title: "My Song",
      artist: "My Artist",
      thumbnailUrl: "https://example.com/thumb.jpg",
      duration: 213,
    });
    const state = get(downloadStore);
    expect(state.state).toBe("awaiting_review");
    expect(state.title).toBe("My Song");
    expect(state.artist).toBe("My Artist");
    expect(state.thumbnailUrl).toBe("https://example.com/thumb.jpg");
    expect(state.duration).toBe(213);
    expect(state.error).toBeNull();
  });

  it("should allow null thumbnailUrl and duration in awaiting_review", () => {
    downloadStore.setAwaitingReview({
      title: "Track",
      artist: "Artist",
      thumbnailUrl: null,
      duration: null,
    });
    const state = get(downloadStore);
    expect(state.state).toBe("awaiting_review");
    expect(state.thumbnailUrl).toBeNull();
    expect(state.duration).toBeNull();
  });

  it("should reset clears all metadata fields", () => {
    downloadStore.setFetchingMetadata("https://example.com/video");
    downloadStore.setAwaitingReview({
      title: "Song",
      artist: "Artist",
      thumbnailUrl: "https://example.com/t.jpg",
      duration: 180,
    });
    downloadStore.reset();
    const state = get(downloadStore);
    expect(state.url).toBeNull();
    expect(state.title).toBeNull();
    expect(state.artist).toBeNull();
    expect(state.thumbnailUrl).toBeNull();
    expect(state.duration).toBeNull();
    expect(state.state).toBe("idle");
  });

  it("should set progress data", () => {
    downloadStore.setProgress({
      percent: 45.5,
      speed: "2.5MiB/s",
      eta: "00:30",
    });
    const state = get(downloadStore);
    expect(state.progress).toEqual({
      percent: 45.5,
      speed: "2.5MiB/s",
      eta: "00:30",
    });
  });

  it("should reset progress when starting download", () => {
    downloadStore.setProgress({
      percent: 45.5,
      speed: "2.5MiB/s",
      eta: "00:30",
    });
    downloadStore.startDownload();
    const state = get(downloadStore);
    expect(state.progress).toBeNull();
  });

  it("should include progress null in reset", () => {
    downloadStore.setProgress({
      percent: 45.5,
      speed: "2.5MiB/s",
      eta: "00:30",
    });
    downloadStore.reset();
    const state = get(downloadStore);
    expect(state.progress).toBeNull();
  });
});
