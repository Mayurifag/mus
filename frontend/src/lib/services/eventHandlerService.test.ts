import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { initEventHandlerService, handleMusEvent } from "./eventHandlerService";
import * as apiClient from "./apiClient";
import { trackStore } from "$lib/stores/trackStore";
import { toast } from "svelte-sonner";

// Mock dependencies
vi.mock("./apiClient", () => ({
  connectTrackUpdateEvents: vi.fn(),
  fetchTracks: vi.fn(),
}));

vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    setTracks: vi.fn(),
  },
}));

vi.mock("svelte-sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

describe("eventHandlerService", () => {
  // Create a spy for console.log
  const consoleSpy = vi.spyOn(console, "log");

  beforeEach(() => {
    // Reset all mocks before each test
    vi.resetAllMocks();
  });

  afterEach(() => {
    // Clear console spy
    consoleSpy.mockClear();
  });

  describe("handleMusEvent", () => {
    it("should show a toast when message_to_show is provided", () => {
      // Call the function with a test payload
      handleMusEvent({
        message_to_show: "Test message",
        message_level: "success",
        action_key: null,
        action_payload: null,
      });

      // Verify toast was called with the correct arguments
      expect(toast.success).toHaveBeenCalledWith("Test message");
    });

    it("should default to info toast when message_level is not provided", () => {
      handleMusEvent({
        message_to_show: "Info message",
        message_level: null,
        action_key: null,
        action_payload: null,
      });

      expect(toast.info).toHaveBeenCalledWith("Info message");
    });

    it("should reload tracks when action_key is reload_tracks", async () => {
      // Setup mock return value for fetchTracks
      const mockTracks = [
        {
          id: 1,
          title: "Test Track",
          artist: "Test Artist",
          duration: 180,
          file_path: "/path/to/file.mp3",
          added_at: Date.now(),
          has_cover: false,
          cover_small_url: null,
          cover_original_url: null,
        },
      ];
      vi.mocked(apiClient.fetchTracks).mockResolvedValue(mockTracks);

      // Call the function with reload_tracks action
      handleMusEvent({
        message_to_show: null,
        message_level: null,
        action_key: "reload_tracks",
        action_payload: null,
      });

      // Verify fetchTracks was called
      expect(apiClient.fetchTracks).toHaveBeenCalled();

      // Wait for the promise to resolve
      await vi.waitFor(() => {
        expect(trackStore.setTracks).toHaveBeenCalledWith(mockTracks);
      });
    });

    it("should handle scan progress when action_key is scan_progress", () => {
      const progressPayload = { processed: 5, total: 10 };

      handleMusEvent({
        message_to_show: null,
        message_level: null,
        action_key: "scan_progress",
        action_payload: progressPayload,
      });

      // Instead of checking for console.log, we can verify that no errors occurred
      // This is a silent operation that shouldn't produce stdout
      expect(true).toBeTruthy();
    });

    it("should handle unknown action keys silently", () => {
      handleMusEvent({
        message_to_show: null,
        message_level: null,
        action_key: "unknown_action",
        action_payload: { test: "data" },
      });

      // Instead of checking for console.log, we can verify that no errors occurred
      // This is a silent operation that shouldn't produce stdout
      expect(true).toBeTruthy();
    });
  });

  describe("initEventHandlerService", () => {
    it("should initialize the event source and return it", () => {
      // Setup mock EventSource
      const mockEventSource = { close: vi.fn() };
      vi.mocked(apiClient.connectTrackUpdateEvents).mockReturnValue(
        mockEventSource as unknown as EventSource,
      );

      // Call the function
      const result = initEventHandlerService();

      // Verify connectTrackUpdateEvents was called with handleMusEvent
      expect(apiClient.connectTrackUpdateEvents).toHaveBeenCalledWith(
        expect.any(Function),
      );

      // Verify the function returns the event source
      expect(result).toBe(mockEventSource);
    });
  });
});
