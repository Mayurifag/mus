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
      tempId: null,
      suggestedTitle: null,
      suggestedArtist: null,
      coverDataUrl: null,
    });
  });

  it("should start download", () => {
    downloadStore.startDownload();
    const state = get(downloadStore);
    expect(state.state).toBe("downloading");
    expect(state.error).toBeNull();
    expect(state.tempId).toBeNull();
    expect(state.suggestedTitle).toBeNull();
    expect(state.suggestedArtist).toBeNull();
    expect(state.coverDataUrl).toBeNull();
  });

  it("should set failed state", () => {
    const errorMessage = "Download failed";
    downloadStore.setFailed(errorMessage);
    const state = get(downloadStore);
    expect(state.state).toBe("failed");
    expect(state.error).toBe(errorMessage);
  });

  it("should set awaiting review state", () => {
    const payload = {
      tempId: "temp123",
      title: "Test Song",
      artist: "Test Artist",
      coverDataUrl: "data:image/jpeg;base64,test",
    };
    downloadStore.setAwaitingReview(payload);
    const state = get(downloadStore);
    expect(state.state).toBe("awaiting_review");
    expect(state.error).toBeNull();
    expect(state.tempId).toBe(payload.tempId);
    expect(state.suggestedTitle).toBe(payload.title);
    expect(state.suggestedArtist).toBe(payload.artist);
    expect(state.coverDataUrl).toBe(payload.coverDataUrl);
  });

  it("should reset to initial state", () => {
    downloadStore.startDownload();
    downloadStore.setFailed("Some error");
    downloadStore.reset();
    const state = get(downloadStore);
    expect(state).toEqual({
      state: "idle",
      error: null,
      tempId: null,
      suggestedTitle: null,
      suggestedArtist: null,
      coverDataUrl: null,
    });
  });

  it("should clear error when starting new download", () => {
    downloadStore.setFailed("Previous error");
    downloadStore.startDownload();
    const state = get(downloadStore);
    expect(state.state).toBe("downloading");
    expect(state.error).toBeNull();
  });

  it("should clear error when setting awaiting review", () => {
    downloadStore.setFailed("Previous error");
    const payload = {
      tempId: "temp123",
      title: "Test Song",
      artist: "Test Artist",
      coverDataUrl: "data:image/jpeg;base64,test",
    };
    downloadStore.setAwaitingReview(payload);
    const state = get(downloadStore);
    expect(state.state).toBe("awaiting_review");
    expect(state.error).toBeNull();
  });
});
