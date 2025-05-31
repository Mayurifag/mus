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

describe("AudioService", () => {
  let audioService: AudioService;
  let mockAudio: HTMLAudioElement;
  let mockOnPlaybackFinished: ReturnType<typeof vi.fn>;

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

    // Create mock callback
    mockOnPlaybackFinished = vi.fn();

    audioService = new AudioService(mockAudio, mockOnPlaybackFinished);
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

  it("should play audio", () => {
    audioService.play();
    expect(mockAudio.play).toHaveBeenCalled();
  });

  it("should pause audio", () => {
    audioService.pause();
    expect(mockAudio.pause).toHaveBeenCalled();
  });

  it("should set volume", () => {
    audioService.setVolume(0.5);
    expect(mockAudio.volume).toBe(0.5);

    audioService.setVolume(0.8);
    expect(mockAudio.volume).toBe(0.8);
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

  it("should toggle repeat state", () => {
    expect(audioService.isRepeat).toBe(false);
    audioService.toggleRepeat();
    expect(audioService.isRepeat).toBe(true);
    audioService.toggleRepeat();
    expect(audioService.isRepeat).toBe(false);
  });

  it("should set repeat state", () => {
    audioService.setRepeat(true);
    expect(audioService.isRepeat).toBe(true);
    audioService.setRepeat(false);
    expect(audioService.isRepeat).toBe(false);
  });

  it("should initialize state with repeat", () => {
    audioService.initializeState(0.8, true, true);
    expect(audioService.volume).toBe(0.8);
    expect(audioService.isMuted).toBe(true);
    expect(audioService.isRepeat).toBe(true);
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

  it("should call onPlaybackFinishedCallback when track ends without repeat", () => {
    audioService.setRepeat(false);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const endedHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "ended",
    )?.[1] as () => void;

    endedHandler();

    expect(mockOnPlaybackFinished).toHaveBeenCalled();
  });

  it("should not call onPlaybackFinishedCallback when track ends with repeat", () => {
    audioService.setRepeat(true);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const endedHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "ended",
    )?.[1] as () => void;

    endedHandler();

    expect(mockOnPlaybackFinished).not.toHaveBeenCalled();
    expect(mockAudio.play).toHaveBeenCalled();
  });

  it("should set document.title when updateAudioSource is called with new track", () => {
    const mockDocument = {
      title: "",
    };
    (globalThis as unknown as { document: typeof mockDocument }).document =
      mockDocument;

    audioService.updateAudioSource(mockTrack, true);

    expect(mockDocument.title).toBe("Test Artist - Test Track");
  });

  it("should not set document.title for same track ID", () => {
    const mockDocument = {
      title: "Initial Title",
    };
    (globalThis as unknown as { document: typeof mockDocument }).document =
      mockDocument;

    audioService.updateAudioSource(mockTrack, true);
    expect(mockDocument.title).toBe("Test Artist - Test Track");

    mockDocument.title = "Changed Title";
    audioService.updateAudioSource(mockTrack, false);

    expect(mockDocument.title).toBe("Changed Title");
  });

  it("should handle updateAudioSource when document is undefined", () => {
    (globalThis as unknown as { document: undefined }).document = undefined;

    expect(() => {
      audioService.updateAudioSource(mockTrack, true);
    }).not.toThrow();
  });

  it("should auto-play when updateAudioSource is called with isPlaying=true", () => {
    audioService.updateAudioSource(mockTrack, true);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const canPlayHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "canplay",
    )?.[1] as () => void;

    canPlayHandler();

    expect(mockAudio.play).toHaveBeenCalled();
  });

  it("should not auto-play when updateAudioSource is called with isPlaying=false", () => {
    audioService.updateAudioSource(mockTrack, false);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const canPlayHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "canplay",
    )?.[1] as () => void;

    canPlayHandler();

    expect(mockAudio.play).not.toHaveBeenCalled();
  });
});
