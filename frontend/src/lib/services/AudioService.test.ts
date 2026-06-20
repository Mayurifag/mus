import { describe, it, expect, vi, beforeEach } from "vitest";
import { AudioService } from "./AudioService";
import type { Track } from "$lib/types";
import { trackStore } from "$lib/stores/trackStore";

const HLS_URL = "https://music.example/api/v1/tracks/1/hls/2001/index.m3u8";
const mockHls = vi.hoisted(() => ({
  isSupported: vi.fn(),
  instances: [] as Array<{
    loadSource: ReturnType<typeof vi.fn>;
    attachMedia: ReturnType<typeof vi.fn>;
    destroy: ReturnType<typeof vi.fn>;
    on: ReturnType<typeof vi.fn>;
  }>,
}));

const mocks = vi.hoisted(() => ({
  toast: {
    info: vi.fn(),
  },
}));

vi.mock("hls.js", () => {
  class MockHls {
    static Events = { ERROR: "hlsError" };
    static isSupported = mockHls.isSupported;
    loadSource = vi.fn();
    attachMedia = vi.fn();
    destroy = vi.fn();
    on = vi.fn();

    constructor() {
      mockHls.instances.push(this);
    }
  }

  return { default: MockHls };
});

vi.mock("svelte-sonner", () => ({
  toast: mocks.toast,
}));

// Mock the trackStore module
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    nextTrack: vi.fn(),
  },
}));

describe("AudioService", () => {
  let audioService: AudioService;
  let mockAudio: HTMLAudioElement;

  const mockTrack: Track = {
    id: "1",
    title: "Test Track",
    artist: "Test Artist",
    duration: 180,
    filename: "test-track.mp3",
    has_cover: true,
    cover_small_url: "/api/v1/tracks/1/covers/small.webp",
    cover_original_url: "/api/v1/tracks/1/covers/original.webp",
    hls_url: HLS_URL,
    updated_at: 1640995200,
  };

  beforeEach(() => {
    mockHls.isSupported.mockReset();
    mockHls.isSupported.mockReturnValue(true);
    mockHls.instances.length = 0;

    // Create a mock audio element
    const mockBuffered = {
      length: 0,
      start: vi.fn(),
      end: vi.fn(),
    };

    mockAudio = {
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      play: vi.fn().mockResolvedValue(undefined),
      pause: vi.fn(),
      load: vi.fn(),
      canPlayType: vi.fn().mockReturnValue(""),
      src: "",
      currentTime: 0,
      duration: 180,
      volume: 1,
      get buffered() {
        return mockBuffered;
      },
    } as unknown as HTMLAudioElement;

    audioService = new AudioService(mockAudio);

    // Reset mock calls
    vi.mocked(trackStore.nextTrack).mockClear();
    mocks.toast.info.mockClear();
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
      "progress",
      expect.any(Function),
    );
    expect(mockAudio.addEventListener).toHaveBeenCalledWith(
      "suspend",
      expect.any(Function),
    );
  });

  it("should update audio source when track changes", () => {
    audioService.updateAudioSource(mockTrack, true);

    const hls = mockHls.instances[0];
    expect(hls.loadSource).toHaveBeenCalledWith(HLS_URL);
    expect(hls.attachMedia).toHaveBeenCalledWith(mockAudio);
    expect(mockAudio.load).not.toHaveBeenCalled();
  });

  it("should not load a new source until playback or seek starts", () => {
    audioService.updateAudioSource(mockTrack, false);

    expect(mockAudio.src).toBe("");
    expect(mockAudio.load).not.toHaveBeenCalled();
    expect(mockHls.instances).toHaveLength(0);

    audioService.play();

    expect(mockHls.instances[0].loadSource).toHaveBeenCalledWith(HLS_URL);
    expect(mockAudio.play).toHaveBeenCalledOnce();
  });

  it("uses native HLS when MediaSource HLS is unavailable", () => {
    mockHls.isSupported.mockReturnValue(false);
    vi.mocked(mockAudio.canPlayType).mockReturnValue("maybe");

    audioService.updateAudioSource(mockTrack, true);

    expect(mockAudio.src).toBe(HLS_URL);
    expect(mockAudio.load).toHaveBeenCalledOnce();
    expect(mockHls.instances).toHaveLength(0);
  });

  it("should request playback when advancing after track end", () => {
    audioService.setRepeat(false);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const endedHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "ended",
    )?.[1] as () => void;

    endedHandler();

    expect(trackStore.nextTrack).toHaveBeenCalledWith(true);
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

  it("should set time immediately", () => {
    mockAudio.currentTime = 0;

    audioService.setTime(10);
    expect(mockAudio.currentTime).toBe(10);

    audioService.setTime(20);
    expect(mockAudio.currentTime).toBe(20);
  });

  it("should handle seeking operations", () => {
    mockAudio.currentTime = 0;

    // Start seeking
    audioService.startSeeking();

    // Seek to a new time (should update store but not audio element)
    audioService.seek(15);
    expect(audioService.currentTime).toBe(15);
    expect(mockAudio.currentTime).toBe(0);

    // End seeking (should update audio element)
    audioService.endSeeking(30);
    expect(audioService.currentTime).toBe(30);
    expect(mockAudio.currentTime).toBe(30);
  });

  it("should ignore timeupdate events during seeking", () => {
    // Set initial time
    audioService.setTime(10);
    expect(audioService.currentTime).toBe(10);

    // Start seeking
    audioService.startSeeking();

    // Seek to a new time
    audioService.seek(20);
    expect(audioService.currentTime).toBe(20);

    // Simulate timeupdate event during seeking
    mockAudio.currentTime = 15;
    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const timeUpdateHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "timeupdate",
    )?.[1] as () => void;

    timeUpdateHandler();

    // Current time should still be the seek value, not the audio element's time
    expect(audioService.currentTime).toBe(20);
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
      "progress",
      expect.any(Function),
    );
    expect(mockAudio.removeEventListener).toHaveBeenCalledWith(
      "suspend",
      expect.any(Function),
    );
  });

  it("should call trackStore.nextTrack when track ends without repeat", () => {
    audioService.setRepeat(false);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const endedHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "ended",
    )?.[1] as () => void;

    endedHandler();

    expect(trackStore.nextTrack).toHaveBeenCalled();
  });

  it("should not call trackStore.nextTrack when track ends with repeat", () => {
    audioService.setRepeat(true);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const endedHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) => call[0] === "ended",
    )?.[1] as () => void;

    endedHandler();

    expect(trackStore.nextTrack).not.toHaveBeenCalled();
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

  it("should format artists correctly in document.title with semicolon-separated artists", () => {
    const mockDocument = {
      title: "",
    };
    (globalThis as unknown as { document: typeof mockDocument }).document =
      mockDocument;

    const trackWithMultipleArtists: Track = {
      ...mockTrack,
      artist: "Artist One;Artist Two;Artist Three",
    };

    audioService.updateAudioSource(trackWithMultipleArtists, true);

    expect(mockDocument.title).toBe(
      "Artist One, Artist Two, Artist Three - Test Track",
    );
  });

  it("should auto-play when updateAudioSource is called with isPlaying=true", () => {
    audioService.updateAudioSource(mockTrack, true);

    expect(mockAudio.play).toHaveBeenCalled();
  });

  it("should show a resume prompt when autoplay is blocked", async () => {
    vi.mocked(mockAudio.play).mockRejectedValue(
      new DOMException("Autoplay blocked", "NotAllowedError"),
    );

    audioService.updateAudioSource(mockTrack, true);
    await Promise.resolve();

    expect(mocks.toast.info).toHaveBeenCalledWith("Tap play to resume");
    expect(mockAudio.pause).toHaveBeenCalled();
  });

  it("should not auto-play when updateAudioSource is called with isPlaying=false", () => {
    audioService.updateAudioSource(mockTrack, false);

    expect(mockAudio.play).not.toHaveBeenCalled();
    expect(mockAudio.load).not.toHaveBeenCalled();
  });

  it("should seek to initial time after metadata loads", () => {
    audioService.updateAudioSource(mockTrack, false, 42);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const loadedMetadataHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) =>
        call[0] === "loadedmetadata" &&
        (call[2] as { once?: boolean } | undefined)?.once,
    )?.[1] as () => void;

    loadedMetadataHandler();

    expect(mockAudio.currentTime).toBe(42);
    expect(mockHls.instances[0].loadSource).toHaveBeenCalledWith(HLS_URL);
  });

  it("should reset new tracks to the beginning after metadata loads", () => {
    mockAudio.currentTime = 99;

    audioService.updateAudioSource(mockTrack, true);

    const addEventListenerMock =
      mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
    const loadedMetadataHandler = addEventListenerMock.mock.calls.find(
      (call: unknown[]) =>
        call[0] === "loadedmetadata" &&
        (call[2] as { once?: boolean } | undefined)?.once,
    )?.[1] as () => void;

    loadedMetadataHandler();

    expect(mockAudio.currentTime).toBe(0);
  });

  describe("buffered ranges", () => {
    function setMockBuffered(mockBufferedData: {
      length: number;
      start: (index: number) => number;
      end: (index: number) => number;
    }) {
      Object.defineProperty(mockAudio, "buffered", {
        value: mockBufferedData,
        configurable: true,
      });
    }

    it("should convert TimeRanges to TimeRange array", () => {
      setMockBuffered({
        length: 2,
        start: (index: number) => (index === 0 ? 0 : 60),
        end: (index: number) => (index === 0 ? 30 : 120),
      });

      const addEventListenerMock =
        mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
      const progressHandler = addEventListenerMock.mock.calls.find(
        (call: unknown[]) => call[0] === "progress",
      )?.[1] as () => void;

      progressHandler();

      const bufferedRanges = audioService.currentBufferedRanges;
      expect(bufferedRanges).toHaveLength(2);
      expect(bufferedRanges[0]).toEqual({ start: 0, end: 30 });
      expect(bufferedRanges[1]).toEqual({ start: 60, end: 120 });
    });

    it("should update buffered ranges on progress event", () => {
      setMockBuffered({
        length: 1,
        start: () => 10,
        end: () => 50,
      });

      const addEventListenerMock =
        mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
      const progressHandler = addEventListenerMock.mock.calls.find(
        (call: unknown[]) => call[0] === "progress",
      )?.[1] as () => void;

      progressHandler();

      expect(audioService.currentBufferedRanges).toEqual([
        { start: 10, end: 50 },
      ]);
    });

    it("should update buffered ranges on suspend event", () => {
      setMockBuffered({
        length: 1,
        start: () => 5,
        end: () => 25,
      });

      const addEventListenerMock =
        mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
      const suspendHandler = addEventListenerMock.mock.calls.find(
        (call: unknown[]) => call[0] === "suspend",
      )?.[1] as () => void;

      suspendHandler();

      expect(audioService.currentBufferedRanges).toEqual([
        { start: 5, end: 25 },
      ]);
    });

    it("should reset buffered ranges when audio source changes", () => {
      setMockBuffered({
        length: 1,
        start: () => 10,
        end: () => 50,
      });

      const addEventListenerMock =
        mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
      const progressHandler = addEventListenerMock.mock.calls.find(
        (call: unknown[]) => call[0] === "progress",
      )?.[1] as () => void;

      progressHandler();
      expect(audioService.currentBufferedRanges).toHaveLength(1);

      audioService.updateAudioSource(mockTrack, false);
      expect(audioService.currentBufferedRanges).toEqual([]);
    });

    it("should provide access to buffered ranges store", () => {
      const store = audioService.currentBufferedRangesStore;
      expect(store).toBeDefined();
      expect(typeof store.subscribe).toBe("function");
    });

    it("should handle empty buffered ranges", () => {
      setMockBuffered({
        length: 0,
        start: () => 0,
        end: () => 0,
      });

      const addEventListenerMock =
        mockAudio.addEventListener as unknown as ReturnType<typeof vi.fn>;
      const progressHandler = addEventListenerMock.mock.calls.find(
        (call: unknown[]) => call[0] === "progress",
      )?.[1] as () => void;

      progressHandler();

      expect(audioService.currentBufferedRanges).toEqual([]);
    });
  });
});
