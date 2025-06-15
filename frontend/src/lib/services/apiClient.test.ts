import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import * as apiClient from "./apiClient";

// Mock the environment module
vi.mock("$app/environment", () => ({
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

const mockTrackTransformed = {
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
  is_shuffle: false,
  is_repeat: false,
};

describe("apiClient", () => {
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

      const result = await apiClient.fetchTracks();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/tracks", {
        headers: {},
      });
      expect(result).toEqual([mockTrackTransformed]);
    });

    it("returns empty array when fetch fails", async () => {
      console.error = vi.fn();
      const mockResponse = {
        ok: false,
        status: 500,
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      const result = await apiClient.fetchTracks();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/tracks", {
        headers: {},
      });
      expect(console.error).toHaveBeenCalledWith(
        "Error fetching tracks:",
        expect.any(Error),
      );
      expect(result).toEqual([]);
    });

    it("returns empty array when network error occurs", async () => {
      console.error = vi.fn();
      vi.mocked(globalThis.fetch).mockRejectedValue(new Error("Network error"));

      const result = await apiClient.fetchTracks();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/tracks", {
        headers: {},
      });
      expect(console.error).toHaveBeenCalledWith(
        "Error fetching tracks:",
        expect.any(Error),
      );
      expect(result).toEqual([]);
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

      const result = await apiClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/player/state", {
        headers: {},
      });
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

      const result = await apiClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/player/state", {
        headers: {},
      });
      expect(result).toEqual({
        current_track_id: null,
        progress_seconds: 0.0,
        volume_level: 1.0,
        is_muted: false,
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

      const result = await apiClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/player/state", {
        headers: {},
      });
      expect(console.error).toHaveBeenCalledWith(
        "Error fetching player state:",
        expect.any(Error),
      );
      expect(result).toEqual({
        current_track_id: null,
        progress_seconds: 0.0,
        volume_level: 1.0,
        is_muted: false,
        is_shuffle: false,
        is_repeat: false,
      });
    });

    it("returns default state when network error occurs", async () => {
      console.error = vi.fn();
      vi.mocked(globalThis.fetch).mockRejectedValue(new Error("Network error"));

      const result = await apiClient.fetchPlayerState();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/player/state", {
        headers: {},
      });
      expect(console.error).toHaveBeenCalledWith(
        "Error fetching player state:",
        expect.any(Error),
      );
      expect(result).toEqual({
        current_track_id: null,
        progress_seconds: 0.0,
        volume_level: 1.0,
        is_muted: false,
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

      apiClient.sendPlayerStateBeacon(mockPlayerState);

      expect(mockSendBeacon).toHaveBeenCalledWith(
        "/api/v1/player/state",
        expect.any(Blob),
      );
    });

    it("handles navigator.sendBeacon not being available", () => {
      vi.stubGlobal("navigator", {});

      expect(() => {
        apiClient.sendPlayerStateBeacon(mockPlayerState);
      }).not.toThrow();

      vi.unstubAllGlobals();
    });
  });

  describe("checkAuthStatus", () => {
    it("returns auth status when fetch is successful", async () => {
      const mockAuthStatus = { auth_enabled: true, authenticated: true };
      const mockResponse = {
        ok: true,
        status: 200,
        json: vi.fn().mockResolvedValue(mockAuthStatus),
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      const result = await apiClient.checkAuthStatus();

      expect(globalThis.fetch).toHaveBeenCalledWith(
        "/api/v1/auth/auth-status",
        {
          headers: {},
        },
      );
      expect(result).toEqual({ authEnabled: true, isAuthenticated: true });
    });

    it("returns default auth status when fetch fails", async () => {
      console.error = vi.fn();
      const mockResponse = {
        ok: false,
        status: 500,
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      const result = await apiClient.checkAuthStatus();

      expect(globalThis.fetch).toHaveBeenCalledWith(
        "/api/v1/auth/auth-status",
        {
          headers: {},
        },
      );
      expect(console.error).toHaveBeenCalledWith(
        "Error checking auth status:",
        expect.any(Error),
      );
      expect(result).toEqual({ authEnabled: false, isAuthenticated: false });
    });
  });

  describe("Authorization headers", () => {
    it("includes Authorization header when auth token is present in localStorage", async () => {
      localStorageMock.getItem.mockReturnValue("test-secret-key");

      const mockResponse = {
        ok: true,
        json: vi.fn().mockResolvedValue([]),
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      await apiClient.fetchTracks();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/tracks", {
        headers: {
          Authorization: "Bearer test-secret-key",
        },
      });
    });

    it("does not include Authorization header when no auth token in localStorage", async () => {
      localStorageMock.getItem.mockReturnValue(null);

      const mockResponse = {
        ok: true,
        json: vi.fn().mockResolvedValue([]),
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      await apiClient.fetchTracks();

      expect(globalThis.fetch).toHaveBeenCalledWith("/api/v1/tracks", {
        headers: {},
      });
    });

    it("includes Authorization header in checkAuthStatus when token is present", async () => {
      localStorageMock.getItem.mockReturnValue("test-secret-key");

      const mockResponse = {
        ok: true,
        json: vi
          .fn()
          .mockResolvedValue({ auth_enabled: true, authenticated: true }),
      };
      vi.mocked(globalThis.fetch).mockResolvedValue(
        mockResponse as unknown as Response,
      );

      await apiClient.checkAuthStatus();

      expect(globalThis.fetch).toHaveBeenCalledWith(
        "/api/v1/auth/auth-status",
        {
          headers: {
            Authorization: "Bearer test-secret-key",
          },
        },
      );
    });
  });
});
