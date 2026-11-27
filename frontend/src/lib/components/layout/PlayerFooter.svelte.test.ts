/**
 * @vitest-environment jsdom
 * @vitest-environment-options { "url": "http://localhost:5173" }
 */
import { vi } from "vitest";

// Mock trackStore
vi.mock("$lib/stores/trackStore", () => {
  const mockStore = {
    subscribe: vi.fn((callback) => {
      callback({
        tracks: [],
        currentTrackIndex: 0,
        playHistory: [],
        currentTrack: {
          id: "1",
          title: "Test Track",
          artist: "Test Artist",
          duration: 180,
          has_cover: true,
          cover_small_url: "/api/v1/tracks/1/covers/small.webp",
          cover_original_url: "/api/v1/tracks/1/covers/original.webp",
          hls_url: "/api/v1/tracks/1/hls/index.m3u8",
        },
        is_shuffle: false,
      });
      return () => {};
    }),
    nextTrack: vi.fn(),
    previousTrack: vi.fn(),
    toggleShuffle: vi.fn(),
  };

  return {
    artistCountsStore: {
      subscribe: vi.fn((callback) => {
        callback({ "Test Artist": 1 });
        return () => {};
      }),
    },
    selectedArtistStore: {
      subscribe: (run: (value: null) => void) => {
        run(null);
        return () => {};
      },
    },
    trackStore: mockStore,
  };
});

import { describe, it, expect, beforeEach } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import type { AudioService } from "$lib/services/AudioService";
import type { TimeRange } from "$lib/types";
import { writable } from "svelte/store";
import PlayerFooter from "./PlayerFooter.svelte";

describe("PlayerFooter component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should show the original cover in the desktop footer", () => {
    const { container } = render(PlayerFooter);
    const cover = container.querySelector('img[alt="Album Cover"]');

    expect(cover?.getAttribute("src")).toBe(
      "/api/v1/tracks/1/covers/original.webp",
    );
  });

  it("shows the artist and title in the mobile footer", () => {
    const { container } = render(PlayerFooter);
    const mobileContainer = container.querySelector(".sm700\\:hidden");
    expect(mobileContainer).toHaveTextContent("Test Artist - Test Track");
  });

  it("should toggle repeat state when repeat button is clicked", async () => {
    const mockIsRepeatStore = {
      subscribe: vi.fn((callback) => {
        callback(false);
        return () => {};
      }),
      set: vi.fn(),
      update: vi.fn(),
    };

    const mockAudioService = {
      isRepeat: false,
      isRepeatStore: mockIsRepeatStore,
      toggleRepeat: vi.fn(),
    } as unknown as AudioService;

    const { container } = render(PlayerFooter, {
      audioService: mockAudioService,
    });

    const repeatButton = container.querySelector(
      'button[aria-label="Toggle Repeat"]',
    );
    expect(repeatButton).toBeTruthy();

    await fireEvent.click(repeatButton as Element);
    expect(mockAudioService.toggleRepeat).toHaveBeenCalledOnce();
  });

  describe("buffered ranges integration", () => {
    it("should pass buffered ranges to progress slider", () => {
      const bufferedRanges: TimeRange[] = [
        { start: 10, end: 50 },
        { start: 80, end: 120 },
      ];

      const mockBufferedRangesStore = writable(bufferedRanges);
      const mockDurationStore = writable(180);
      const mockCurrentTimeStore = writable(30);
      const mockIsPlayingStore = writable(false);

      const mockAudioService = {
        currentBufferedRangesStore: mockBufferedRangesStore,
        durationStore: mockDurationStore,
        currentTimeStore: mockCurrentTimeStore,
        isPlayingStore: mockIsPlayingStore,
      } as unknown as AudioService;

      const { container } = render(PlayerFooter, {
        audioService: mockAudioService,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(4);
    });

    it("should work without audioService", () => {
      const { container } = render(PlayerFooter);
      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });
  });
});
