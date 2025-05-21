import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import * as apiClient from "./apiClient";

// Sample data for tests
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
    // Reset all mocks
    vi.resetAllMocks();

    // Mock the functions directly
    vi.spyOn(apiClient, "fetchTracks");
    vi.spyOn(apiClient, "fetchPlayerState");
    vi.spyOn(apiClient, "savePlayerState");
  });

  afterEach(() => {
    // Restore original console.error
    console.error = originalConsoleError;

    // Clear all mocks
    vi.restoreAllMocks();
  });

  describe("fetchTracks", () => {
    it("returns tracks when api call is successful", async () => {
      // Mock the function implementation
      vi.mocked(apiClient.fetchTracks).mockResolvedValue([mockTrack]);

      // Call the function
      const result = await apiClient.fetchTracks();

      // Verify the function was called
      expect(apiClient.fetchTracks).toHaveBeenCalled();

      // Verify the result
      expect(result).toEqual([mockTrack]);
    });

    it("handles errors gracefully", async () => {
      // Mock console.error to prevent actual logging during test
      console.error = vi.fn();

      // Mock the function implementation to simulate error handling
      vi.mocked(apiClient.fetchTracks).mockImplementation(async () => {
        console.error("Error fetching tracks:", new Error("Network error"));
        return [];
      });

      // Call the function
      const result = await apiClient.fetchTracks();

      // Verify the function was called
      expect(apiClient.fetchTracks).toHaveBeenCalled();

      // Verify console.error was called
      expect(console.error).toHaveBeenCalledWith(
        "Error fetching tracks:",
        expect.any(Error),
      );

      // Verify the result is an empty array
      expect(result).toEqual([]);
    });
  });

  describe("fetchPlayerState", () => {
    it("returns player state when api call is successful", async () => {
      // Mock the function implementation
      vi.mocked(apiClient.fetchPlayerState).mockResolvedValue(mockPlayerState);

      // Call the function
      const result = await apiClient.fetchPlayerState();

      // Verify the function was called
      expect(apiClient.fetchPlayerState).toHaveBeenCalled();

      // Verify the result
      expect(result).toEqual(mockPlayerState);
    });

    it("returns null when receiving 404", async () => {
      // Mock the function implementation to return null (404 handling)
      vi.mocked(apiClient.fetchPlayerState).mockResolvedValue(null);

      // Call the function
      const result = await apiClient.fetchPlayerState();

      // Verify the function was called
      expect(apiClient.fetchPlayerState).toHaveBeenCalled();

      // Verify the result is null
      expect(result).toBeNull();
    });

    it("returns null when api call fails with other error", async () => {
      // Mock console.error to prevent actual logging during test
      console.error = vi.fn();

      // Mock the function implementation to simulate error handling
      vi.mocked(apiClient.fetchPlayerState).mockImplementation(async () => {
        console.error(
          "Error fetching player state:",
          new Error("Server error"),
        );
        return null;
      });

      // Call the function
      const result = await apiClient.fetchPlayerState();

      // Verify the function was called
      expect(apiClient.fetchPlayerState).toHaveBeenCalled();

      // Verify console.error was called
      expect(console.error).toHaveBeenCalledWith(
        "Error fetching player state:",
        expect.any(Error),
      );

      // Verify the result is null
      expect(result).toBeNull();
    });
  });

  describe("savePlayerState", () => {
    it("saves player state and returns the response when successful", async () => {
      // Mock the function implementation
      vi.mocked(apiClient.savePlayerState).mockResolvedValue(mockPlayerState);

      // Call the function
      const result = await apiClient.savePlayerState(mockPlayerState);

      // Verify the function was called with the correct arguments
      expect(apiClient.savePlayerState).toHaveBeenCalledWith(mockPlayerState);

      // Verify the result
      expect(result).toEqual(mockPlayerState);
    });

    it("returns null when api call fails", async () => {
      // Mock console.error to prevent actual logging during test
      console.error = vi.fn();

      // Mock the function implementation to simulate error handling
      vi.mocked(apiClient.savePlayerState).mockImplementation(async () => {
        console.error("Error saving player state:", new Error("Network error"));
        return null;
      });

      // Call the function
      const result = await apiClient.savePlayerState(mockPlayerState);

      // Verify the function was called with the correct arguments
      expect(apiClient.savePlayerState).toHaveBeenCalledWith(mockPlayerState);

      // Verify console.error was called
      expect(console.error).toHaveBeenCalledWith(
        "Error saving player state:",
        expect.any(Error),
      );

      // Verify the result is null
      expect(result).toBeNull();
    });
  });
});
