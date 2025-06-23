import { vi, describe, it, expect, beforeEach } from "vitest";
import { load } from "../+layout.server";

// Mock the SvelteKit modules
vi.mock("@sveltejs/kit", () => ({
  error: vi.fn((code, opts) => ({ status: code, ...opts })),
}));

// Mock the API client functions
vi.mock("$lib/services/apiClient", () => ({
  fetchTracks: vi.fn(),
  fetchPlayerState: vi.fn(),
}));

import { error } from "@sveltejs/kit";
import { fetchTracks, fetchPlayerState } from "$lib/services/apiClient";

describe("+layout.server.ts", () => {
  const mockTracks = [
    {
      id: 1,
      title: "Test Track",
      artist: "Test Artist",
      duration: 180,
      has_cover: true,
      cover_small_url: "/api/v1/tracks/1/covers/small.webp",
      cover_original_url: "/api/v1/tracks/1/covers/original.webp",
    },
  ];

  const mockPlayerState = {
    current_track_id: 1,
    progress_seconds: 30,
    volume_level: 0.7,
    is_muted: false,
    is_shuffle: false,
    is_repeat: false,
  };

  // Mock event object required by the load function
  const mockEvent = {
    fetch: vi.fn(),
  } as unknown as Parameters<typeof load>[0];

  beforeEach(() => {
    vi.resetAllMocks();
    // Suppress console.error for these tests
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("loads tracks and player state successfully", async () => {
    vi.mocked(fetchTracks).mockResolvedValue(mockTracks);
    vi.mocked(fetchPlayerState).mockResolvedValue(mockPlayerState);

    const result = await load(mockEvent);

    expect(fetchTracks).toHaveBeenCalled();
    expect(fetchPlayerState).toHaveBeenCalled();
    expect(result).toEqual({
      tracks: mockTracks,
      playerState: mockPlayerState,
    });
  });

  it("handles default player state when none exists", async () => {
    const defaultPlayerState = {
      current_track_id: null,
      progress_seconds: 0.0,
      volume_level: 1.0,
      is_muted: false,
      is_shuffle: false,
      is_repeat: false,
    };

    vi.mocked(fetchTracks).mockResolvedValue(mockTracks);
    vi.mocked(fetchPlayerState).mockResolvedValue(defaultPlayerState);

    const result = await load(mockEvent);

    expect(fetchTracks).toHaveBeenCalled();
    expect(fetchPlayerState).toHaveBeenCalled();
    expect(result).toEqual({
      tracks: mockTracks,
      playerState: defaultPlayerState,
    });
  });

  it("handles empty tracks array", async () => {
    vi.mocked(fetchTracks).mockResolvedValue([]);
    vi.mocked(fetchPlayerState).mockResolvedValue(mockPlayerState);

    const result = await load(mockEvent);

    expect(fetchTracks).toHaveBeenCalled();
    expect(fetchPlayerState).toHaveBeenCalled();
    expect(result).toEqual({
      tracks: [],
      playerState: mockPlayerState,
    });
  });

  it("throws error on API failure", async () => {
    const testError = new Error("API failure");
    vi.mocked(fetchTracks).mockRejectedValue(testError);
    vi.mocked(fetchPlayerState).mockResolvedValue(mockPlayerState);

    await expect(() => load(mockEvent)).rejects.toThrow();
    expect(error).toHaveBeenCalledWith(500, {
      message: "Failed to load initial data",
    });
  });

  it("handles rejection of fetchPlayerState", async () => {
    const testError = new Error("API failure");
    vi.mocked(fetchTracks).mockResolvedValue(mockTracks);
    vi.mocked(fetchPlayerState).mockRejectedValue(testError);

    await expect(() => load(mockEvent)).rejects.toThrow();
    expect(error).toHaveBeenCalledWith(500, {
      message: "Failed to load initial data",
    });
  });
});
