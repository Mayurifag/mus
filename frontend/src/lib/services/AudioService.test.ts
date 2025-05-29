import { describe, it, expect, vi, beforeEach } from "vitest";
import { AudioService } from "./AudioService";
import type { Track } from "$lib/types";

// Mock the apiClient module
vi.mock("$lib/services/apiClient", () => ({
  getStreamUrl: vi.fn(
    (trackId: number) =>
      `http://localhost:8000/api/v1/tracks/${trackId}/stream`,
  ),
}));

interface MockPlayerStore {
  subscribe: ReturnType<typeof vi.fn>;
  setCurrentTime: ReturnType<typeof vi.fn>;
  setDuration: ReturnType<typeof vi.fn>;
  pause: ReturnType<typeof vi.fn>;
}

interface MockTrackStore {
  nextTrack: ReturnType<typeof vi.fn>;
}

describe("AudioService", () => {
  let audioService: AudioService;
  let mockAudio: HTMLAudioElement;
  let mockPlayerStore: MockPlayerStore;
  let mockTrackStore: MockTrackStore;

  const mockTrack: Track = {
    id: 1,
    title: "Test Track",
    artist: "Test Artist",
    duration: 180,
    file_path: "/path/to/file.mp3",
    added_at: 1615478400,
    has_cover: true,
    cover_small_url: "/api/v1/tracks/1/covers/small.webp",
    cover_original_url: "/api/v1/tracks/1/covers/original.webp",
  };

  beforeEach(() => {
    // Create a mock audio element
    mockAudio = {
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      play: vi.fn().mockResolvedValue(undefined),
      pause: vi.fn(),
      load: vi.fn(),
      src: "",
      currentTime: 0,
      duration: 180,
      volume: 1,
    } as unknown as HTMLAudioElement;

    // Create mock stores
    mockPlayerStore = {
      subscribe: vi.fn(),
      setCurrentTime: vi.fn(),
      setDuration: vi.fn(),
      pause: vi.fn(),
    };

    mockTrackStore = {
      nextTrack: vi.fn(),
    };

    // Mock get function for store state
    vi.doMock("svelte/store", () => ({
      get: vi.fn(() => ({
        is_repeat: false,
      })),
    }));

    audioService = new AudioService(mockAudio, mockPlayerStore, mockTrackStore);
  });

  it("should set up event listeners on construction", () => {
    expect(mockAudio.addEventListener).toHaveBeenCalledWith(
      "loadedmetadata",
      expect.any(Function),
    );
    expect(mockAudio.addEventListener).toHaveBeenCalledWith(
      "timeupdate",
      expect.any(Function),
    );
    expect(mockAudio.addEventListener).toHaveBeenCalledWith(
      "ended",
      expect.any(Function),
    );
    expect(mockAudio.addEventListener).toHaveBeenCalledWith(
      "error",
      expect.any(Function),
    );
    expect(mockAudio.addEventListener).toHaveBeenCalledWith(
      "canplay",
      expect.any(Function),
    );
  });

  it("should update audio source when track changes", () => {
    audioService.updateAudioSource(mockTrack, true);

    expect(mockAudio.src).toBe("http://localhost:8000/api/v1/tracks/1/stream");
    expect(mockAudio.load).toHaveBeenCalled();
  });

  it("should not update audio source if track is null", () => {
    audioService.updateAudioSource(null, true);

    expect(mockAudio.src).toBe("");
    expect(mockAudio.load).not.toHaveBeenCalled();
  });

  it("should play audio", async () => {
    await audioService.play();
    expect(mockAudio.play).toHaveBeenCalled();
  });

  it("should pause audio", () => {
    audioService.pause();
    expect(mockAudio.pause).toHaveBeenCalled();
  });

  it("should set volume", () => {
    audioService.setVolume(0.5, false);
    expect(mockAudio.volume).toBe(0.5);

    audioService.setVolume(0.8, true);
    expect(mockAudio.volume).toBe(0);
  });

  it("should set current time with debouncing", () => {
    mockAudio.currentTime = 0;

    // First call should work
    audioService.setCurrentTime(10);
    expect(mockAudio.currentTime).toBe(10);

    // Immediate second call should be debounced
    audioService.setCurrentTime(20);
    expect(mockAudio.currentTime).toBe(10); // Should not change due to debouncing
  });

  it("should clean up event listeners on destroy", () => {
    audioService.destroy();

    expect(mockAudio.removeEventListener).toHaveBeenCalledWith(
      "loadedmetadata",
      expect.any(Function),
    );
    expect(mockAudio.removeEventListener).toHaveBeenCalledWith(
      "timeupdate",
      expect.any(Function),
    );
    expect(mockAudio.removeEventListener).toHaveBeenCalledWith(
      "ended",
      expect.any(Function),
    );
    expect(mockAudio.removeEventListener).toHaveBeenCalledWith(
      "error",
      expect.any(Function),
    );
    expect(mockAudio.removeEventListener).toHaveBeenCalledWith(
      "canplay",
      expect.any(Function),
    );
  });
});
