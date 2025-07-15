import { describe, it, expect, vi, beforeEach } from "vitest";
import { initEventHandlerService, handleMusEvent } from "./eventHandlerService";
import * as apiClient from "./apiClient";
import { trackStore } from "$lib/stores/trackStore";
import { recentEventsStore } from "$lib/stores/recentEventsStore";
import type { MusEvent, Track } from "$lib/types";

// Mock dependencies
vi.mock("./apiClient", () => ({
  connectTrackUpdateEvents: vi.fn(),
  createTrackWithUrls: vi.fn(),
}));

vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    addTrack: vi.fn(),
    updateTrack: vi.fn(),
    deleteTrack: vi.fn(),
  },
}));

vi.mock("$lib/stores/recentEventsStore", () => ({
  recentEventsStore: {
    addEvent: vi.fn(),
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

// Mock environment variables
vi.mock("$app/environment", () => ({
  browser: true,
}));

describe("eventHandlerService", () => {
  beforeEach(async () => {
    vi.clearAllMocks();

    // Set up createTrackWithUrls mock to return the input data
    const apiClient = await import("./apiClient");
    vi.mocked(apiClient.createTrackWithUrls).mockImplementation(
      (data) => data as Track,
    );
  });

  describe("handleMusEvent", () => {
    it("should show toast message when message_to_show is provided", async () => {
      const { toast } = await import("svelte-sonner");

      const payload: MusEvent = {
        message_to_show: "Test message",
        message_level: "success",
        action_key: null,
        action_payload: null,
      };

      handleMusEvent(payload);

      expect(toast.success).toHaveBeenCalledWith("Test message");
      expect(recentEventsStore.addEvent).toHaveBeenCalledWith(payload);
    });

    it("should default to info level when message_level is null", async () => {
      const { toast } = await import("svelte-sonner");

      const payload: MusEvent = {
        message_to_show: "Test message",
        message_level: null,
        action_key: null,
        action_payload: null,
      };

      handleMusEvent(payload);

      expect(toast.info).toHaveBeenCalledWith("Test message");
      expect(recentEventsStore.addEvent).toHaveBeenCalledWith(payload);
    });

    it("should add event to recent events store even without message", () => {
      const payload: MusEvent = {
        message_to_show: null,
        message_level: null,
        action_key: "track_updated",
        action_payload: null,
      };

      handleMusEvent(payload);

      expect(recentEventsStore.addEvent).toHaveBeenCalledWith(payload);
    });

    it("should handle track_added event", () => {
      const trackData = {
        id: 1,
        title: "Test Track",
        artist: "Test Artist",
        duration: 180,
        file_path: "/music/test.mp3",
        has_cover: true,
        cover_small_url: "/api/v1/tracks/1/covers/small.webp",
        cover_original_url: "/api/v1/tracks/1/covers/original.webp",
      };

      const payload: MusEvent = {
        message_to_show: null,
        message_level: null,
        action_key: "track_added",
        action_payload: trackData,
      };

      handleMusEvent(payload);

      expect(trackStore.addTrack).toHaveBeenCalledWith({
        ...trackData,
        cover_small_url: "/api/v1/tracks/1/covers/small.webp",
        cover_original_url: "/api/v1/tracks/1/covers/original.webp",
      });
    });

    it("should handle track_updated event", () => {
      const trackData = {
        id: 1,
        title: "Updated Track",
        artist: "Updated Artist",
        duration: 200,
        file_path: "/music/updated.mp3",
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
      };

      const payload: MusEvent = {
        message_to_show: null,
        message_level: null,
        action_key: "track_updated",
        action_payload: trackData,
      };

      handleMusEvent(payload);

      expect(trackStore.updateTrack).toHaveBeenCalledWith({
        ...trackData,
        cover_small_url: null,
        cover_original_url: null,
      });
    });

    it("should handle track_deleted event", () => {
      const payload: MusEvent = {
        message_to_show: null,
        message_level: null,
        action_key: "track_deleted",
        action_payload: { id: 1 },
      };

      handleMusEvent(payload);

      expect(trackStore.deleteTrack).toHaveBeenCalledWith(1);
    });

    it("should not call trackStore methods when action_payload is missing", () => {
      const payload: MusEvent = {
        message_to_show: null,
        message_level: null,
        action_key: "track_added",
        action_payload: null,
      };

      handleMusEvent(payload);

      expect(trackStore.addTrack).not.toHaveBeenCalled();
      expect(trackStore.updateTrack).not.toHaveBeenCalled();
      expect(trackStore.deleteTrack).not.toHaveBeenCalled();
    });

    it("should not call deleteTrack when id is not a number", () => {
      const payload: MusEvent = {
        message_to_show: null,
        message_level: null,
        action_key: "track_deleted",
        action_payload: { id: "not-a-number" },
      };

      handleMusEvent(payload);

      expect(trackStore.deleteTrack).not.toHaveBeenCalled();
    });

    it("should handle unknown action_key gracefully", () => {
      const payload: MusEvent = {
        message_to_show: null,
        message_level: null,
        action_key: "unknown_action",
        action_payload: null,
      };

      expect(() => handleMusEvent(payload)).not.toThrow();
    });
  });

  describe("initEventHandlerService", () => {
    it("should initialize the event source and return it", () => {
      const mockEventSource = { close: vi.fn() };
      vi.mocked(apiClient.connectTrackUpdateEvents).mockReturnValue(
        mockEventSource as unknown as EventSource,
      );

      const result = initEventHandlerService();

      expect(apiClient.connectTrackUpdateEvents).toHaveBeenCalledWith(
        expect.any(Function),
      );
      expect(result).toBe(mockEventSource);
    });
  });
});
