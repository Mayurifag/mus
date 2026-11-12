import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import * as trackClient from "./trackClient";
import * as playerClient from "./playerClient";

const PUBLIC_API_HOST = import.meta.env.VITE_PUBLIC_API_HOST || "";
const API_HOST = import.meta.env.SSR
  ? import.meta.env.VITE_INTERNAL_API_HOST || ""
  : PUBLIC_API_HOST;
const API_BASE_URL = `${API_HOST}/api/v1`;
const PUBLIC_API_BASE_URL = `${PUBLIC_API_HOST}/api/v1`;
const TRACKS_URL = `${API_BASE_URL}/tracks`;
const PLAYER_STATE_URL = `${API_BASE_URL}/player/state`;

// Mock the environment module
vi.mock("$app/env", () => ({
  dev: true,
  browser: true,
}));

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(globalThis, "localStorage", {
  value: localStorageMock,
  writable: true,
});

const mockTrackFromBackend = {
  id: "1",
  title: "Test Track",
  artist: "Test Artist",
  duration: 180,
  updated_at: 1640995200,
  has_cover: true,
  cover_small_url: "/api/v1/tracks/1/covers/small.webp",
  cover_original_url: "/api/v1/tracks/1/covers/original.webp",
  hls_url: "/api/v1/tracks/1/hls/1640995200/index.m3u8",
};

const mockTrackTransformed = {
  id: "1",
  title: "Test Track",
  artist: "Test Artist",
  duration: 180,
  updated_at: 1640995200,
  has_cover: true,
  cover_small_url: `${PUBLIC_API_BASE_URL}/tracks/1/covers/small.webp?v=1640995200`,
  cover_original_url: `${PUBLIC_API_BASE_URL}/tracks/1/covers/original.webp?v=1640995200`,
  hls_url: `${PUBLIC_API_BASE_URL}/tracks/1/hls/1640995200/index.m3u8`,
  tags: [],
};

const mockPlayerState = {
  current_track_id: "1",
  progress_seconds: 30,
  volume_level: 0.7,
  is_muted: false,
  is_playing: false,
  is_shuffle: false,
  is_repeat: false,
};

describe("API clients", () => {
  const originalConsoleError = console.error;
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    vi.resetAllMocks();
    globalThis.fetch = vi.fn();
    localStorageMock.getItem.mockReturnValue(null);
  });

  afterEach(() => {
    console.error = originalConsoleError;
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  describe("fetchTracks", () => {
    it("returns tracks when fetch is successful", async () => {
      const mockResponse = {
        ok: true,
        json: vi.fn().mockResolvedValue([mockTrackFromBackend]),
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      const result = await trackClient.fetchTracks();

      expect(globalThis.fetch).toHaveBeenCalledWith(TRACKS_URL);
      expect(result).toEqual([mockTrackTransformed]);
    });

    it("rejects failed refreshes instead of treating them as an empty library", async () => {
      console.error = vi.fn();
      const mockResponse = {
        ok: false,
        status: 500,
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      await expect(trackClient.fetchTracks()).rejects.toThrow(
        "HTTP error! status: 500",
      );
      expect(globalThis.fetch).toHaveBeenCalledWith(TRACKS_URL);
    });

    it("preserves network failures for the caller", async () => {
      console.error = vi.fn();
      const error = new Error("Network error");
      vi.mocked(globalThis.fetch).mockRejectedValue(error);
      await expect(trackClient.fetchTracks()).rejects.toBe(error);
      expect(globalThis.fetch).toHaveBeenCalledWith(TRACKS_URL);
    });
  });

  describe("fetchPlayerState", () => {
    it("returns player state when fetch is successful", async () => {
      const mockResponse = {
        ok: true,
        status: 200,
        json: vi.fn().mockResolvedValue(mockPlayerState),
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      const result = await playerClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith(PLAYER_STATE_URL);
      expect(result).toEqual(mockPlayerState);
    });

    it("returns default state when receiving 404", async () => {
      console.error = vi.fn();
      const mockResponse = {
        ok: false,
        status: 404,
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      const result = await playerClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith(PLAYER_STATE_URL);
      expect(result).toEqual({
        current_track_id: null,
        progress_seconds: 0.0,
        volume_level: 1.0,
        is_muted: false,
        is_playing: false,
        is_shuffle: false,
        is_repeat: false,
      });
    });

    it("returns default state when fetch fails with other error", async () => {
      console.error = vi.fn();
      const mockResponse = {
        ok: false,
        status: 500,
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      const result = await playerClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith(PLAYER_STATE_URL);
      expect(console.error).toHaveBeenCalledWith(
        expect.stringContaining("fetchPlayerState"),
        expect.any(Error),
      );
      expect(result).toEqual({
        current_track_id: null,
        progress_seconds: 0.0,
        volume_level: 1.0,
        is_muted: false,
        is_playing: false,
        is_shuffle: false,
        is_repeat: false,
      });
    });

    it("returns default state when network error occurs", async () => {
      console.error = vi.fn();
      vi.mocked(globalThis.fetch).mockRejectedValue(new Error("Network error"));

      const result = await playerClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith(PLAYER_STATE_URL);
      expect(console.error).toHaveBeenCalledWith(
        expect.stringContaining("fetchPlayerState"),
        expect.any(Error),
      );
      expect(result).toEqual({
        current_track_id: null,
        progress_seconds: 0.0,
        volume_level: 1.0,
        is_muted: false,
        is_playing: false,
        is_shuffle: false,
        is_repeat: false,
      });
    });
  });

  describe("sendPlayerStateBeacon", () => {
    it("calls navigator.sendBeacon when available", () => {
      const mockSendBeacon = vi.fn().mockReturnValue(true);
      Object.defineProperty(navigator, "sendBeacon", {
        value: mockSendBeacon,
        writable: true,
      });

      playerClient.sendPlayerStateBeacon(mockPlayerState);

      expect(mockSendBeacon).toHaveBeenCalledWith(
        PLAYER_STATE_URL,
        expect.any(Blob),
      );
    });

    it("handles navigator.sendBeacon not being available", () => {
      vi.stubGlobal("navigator", {});

      expect(() => {
        playerClient.sendPlayerStateBeacon(mockPlayerState);
      }).not.toThrow();

      vi.unstubAllGlobals();
    });
  });
});
