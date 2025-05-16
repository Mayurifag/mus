import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import {
  fetchTracks,
  fetchPlayerState,
  savePlayerState,
  triggerScan,
} from "./apiClient";

// Mock fetch
vi.stubGlobal("fetch", vi.fn());

const mockTrack = {
  id: 1,
  title: "Test Track",
  artist: "Test Artist",
  duration: 180,
  file_path: "/path/to/song.mp3",
  added_at: Math.floor(Date.now() / 1000) - 3600,
  has_cover: true,
  cover_small_url: "/api/v1/tracks/1/covers/small.webp",
  cover_original_url: "/api/v1/tracks/1/covers/original.webp",
};

const mockPlayerState = {
  current_track_id: 1,
  progress_seconds: 30,
  volume_level: 0.7,
  is_muted: false,
};

describe("apiClient", () => {
  // Save original console.error
  const originalConsoleError = console.error;

  beforeEach(() => {
    vi.resetAllMocks();
  });

  afterEach(() => {
    // Restore original console.error after each test
    console.error = originalConsoleError;
  });

  describe("fetchTracks", () => {
    it("returns tracks when fetch is successful", async () => {
      // Mock successful response
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue([mockTrack]),
      } as unknown as Response);

      const result = await fetchTracks();

      expect(fetch).toHaveBeenCalledWith(expect.stringContaining("/tracks"));
      expect(result).toEqual([mockTrack]);
    });

    it("returns empty array when fetch fails", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock failed response
      vi.mocked(fetch).mockResolvedValue({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
      } as unknown as Response);

      const result = await fetchTracks();

      expect(fetch).toHaveBeenCalledWith(expect.stringContaining("/tracks"));
      expect(result).toEqual([]);
      expect(console.error).toHaveBeenCalled();
    });

    it("handles fetch rejection gracefully", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock fetch rejection
      vi.mocked(fetch).mockRejectedValue(new Error("Network error"));

      const result = await fetchTracks();

      expect(fetch).toHaveBeenCalledWith(expect.stringContaining("/tracks"));
      expect(result).toEqual([]);
      expect(console.error).toHaveBeenCalled();
    });
  });

  describe("fetchPlayerState", () => {
    it("returns player state when fetch is successful", async () => {
      // Mock successful response
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(mockPlayerState),
      } as unknown as Response);

      const result = await fetchPlayerState();

      expect(fetch).toHaveBeenCalledWith(
        expect.stringContaining("/player/state"),
      );
      expect(result).toEqual(mockPlayerState);
    });

    it("returns null when receiving 404", async () => {
      // Mock 404 response
      vi.mocked(fetch).mockResolvedValue({
        ok: false,
        status: 404,
      } as unknown as Response);

      const result = await fetchPlayerState();

      expect(fetch).toHaveBeenCalledWith(
        expect.stringContaining("/player/state"),
      );
      expect(result).toBeNull();
    });

    it("returns null when fetch fails with non-404 error", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock failed response
      vi.mocked(fetch).mockResolvedValue({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
      } as unknown as Response);

      const result = await fetchPlayerState();

      expect(fetch).toHaveBeenCalledWith(
        expect.stringContaining("/player/state"),
      );
      expect(result).toBeNull();
      expect(console.error).toHaveBeenCalled();
    });

    it("handles fetch rejection gracefully", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock fetch rejection
      vi.mocked(fetch).mockRejectedValue(new Error("Network error"));

      const result = await fetchPlayerState();

      expect(fetch).toHaveBeenCalledWith(
        expect.stringContaining("/player/state"),
      );
      expect(result).toBeNull();
      expect(console.error).toHaveBeenCalled();
    });
  });

  describe("savePlayerState", () => {
    it("saves player state and returns the response when successful", async () => {
      // Mock successful response
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(mockPlayerState),
      } as unknown as Response);

      const result = await savePlayerState(mockPlayerState);

      expect(fetch).toHaveBeenCalledWith(
        expect.stringContaining("/player/state"),
        expect.objectContaining({
          method: "POST",
          headers: expect.objectContaining({
            "Content-Type": "application/json",
          }),
          body: JSON.stringify(mockPlayerState),
        }),
      );
      expect(result).toEqual(mockPlayerState);
    });

    it("returns null when fetch fails", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock failed response
      vi.mocked(fetch).mockResolvedValue({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
      } as unknown as Response);

      const result = await savePlayerState(mockPlayerState);

      expect(fetch).toHaveBeenCalled();
      expect(result).toBeNull();
      expect(console.error).toHaveBeenCalled();
    });

    it("handles fetch rejection gracefully", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock fetch rejection
      vi.mocked(fetch).mockRejectedValue(new Error("Network error"));

      const result = await savePlayerState(mockPlayerState);

      expect(fetch).toHaveBeenCalled();
      expect(result).toBeNull();
      expect(console.error).toHaveBeenCalled();
    });
  });

  describe("triggerScan", () => {
    it("returns success response when scan succeeds", async () => {
      const successResponse = {
        success: true,
        message: "Scan started successfully",
      };

      // Mock successful response
      vi.mocked(fetch).mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue(successResponse),
      } as unknown as Response);

      const result = await triggerScan();

      expect(fetch).toHaveBeenCalledWith(
        expect.stringContaining("/scan"),
        expect.objectContaining({
          method: "POST",
        }),
      );
      expect(result).toEqual(successResponse);
    });

    it("returns error object when fetch fails", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock failed response
      vi.mocked(fetch).mockResolvedValue({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
      } as unknown as Response);

      const result = await triggerScan();

      expect(fetch).toHaveBeenCalled();
      expect(result).toEqual({
        success: false,
        message: "Failed to trigger scan: 500 Internal Server Error",
      });
      expect(console.error).toHaveBeenCalled();
    });

    it("handles fetch rejection gracefully", async () => {
      // Silence console.error for this test
      console.error = vi.fn();

      // Mock fetch rejection with a specific error message
      const errorMessage = "Network error";
      vi.mocked(fetch).mockRejectedValue(new Error(errorMessage));

      const result = await triggerScan();

      expect(fetch).toHaveBeenCalled();
      expect(result).toEqual({
        success: false,
        message: errorMessage,
      });
      expect(console.error).toHaveBeenCalled();
    });
  });
});
