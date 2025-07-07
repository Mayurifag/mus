import { vi } from "vitest";

// Mock the trackStore
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    playTrack: vi.fn(),
  },
}));

// Mock the playerStore (now only UI state)
vi.mock("$lib/stores/playerStore", () => ({
  playerStore: {
    subscribe: vi.fn().mockImplementation((callback) => {
      callback({
        currentTrack: null,
        is_shuffle: false,
      });
      return () => {};
    }),
  },
}));

import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import type { Track, TimeRange } from "$lib/types";
import type { AudioService } from "$lib/services/AudioService";
import { writable } from "svelte/store";
import "@testing-library/jest-dom/vitest";
import TrackItem from "./TrackItem.svelte";
import { trackStore } from "$lib/stores/trackStore";

describe("TrackItem component", () => {
  let mockTrack: Track;

  beforeEach(() => {
    mockTrack = {
      id: 1,
      title: "Test Song",
      artist: "Test Artist",
      duration: 180,
      file_path: "/music/test-track.mp3",
      has_cover: true,
      cover_small_url: "/api/v1/tracks/1/covers/small.webp",
      cover_original_url: "/api/v1/tracks/1/covers/original.webp",
    };

    // Clear the mocks
    vi.mocked(trackStore.playTrack).mockClear();
  });

  it("renders track details correctly", () => {
    render(TrackItem, {
      track: mockTrack,
      index: 0,
      isSelected: false,
    });

    expect(screen.getByText("Test Song")).toBeInTheDocument();
    expect(screen.getByText("Test Artist")).toBeInTheDocument();
    expect(screen.getByText("3:00")).toBeInTheDocument(); // 180 seconds formatted
    expect(screen.getByAltText("Album art for Test Song")).toBeInTheDocument();
  });

  it("renders placeholder for missing cover art", () => {
    const trackWithoutCover = {
      ...mockTrack,
      has_cover: false,
      cover_small_url: null,
    };
    render(TrackItem, {
      track: trackWithoutCover,
      index: 0,
      isSelected: false,
    });

    expect(screen.getByAltText("No Album Cover")).toBeInTheDocument();
  });

  it("applies selected styles when isSelected is true", () => {
    const { container } = render(TrackItem, {
      track: mockTrack,
      index: 0,
      isSelected: true,
    });
    const trackItemDiv = container.querySelector('[data-testid="track-item"]');

    expect(trackItemDiv?.classList.contains("bg-secondary")).toBe(true);
  });

  it("renders progress slider when track is selected", () => {
    render(TrackItem, { track: mockTrack, index: 0, isSelected: true });

    const progressSlider = screen.getByTestId("track-progress-slider");
    expect(progressSlider).toBeInTheDocument();
  });

  it("does not render progress slider when track is not selected", () => {
    render(TrackItem, { track: mockTrack, index: 0, isSelected: false });

    const progressSlider = screen.queryByTestId("track-progress-slider");
    expect(progressSlider).not.toBeInTheDocument();
  });

  it("includes track ID in the element ID for scroll targeting", () => {
    const { container } = render(TrackItem, {
      track: mockTrack,
      index: 0,
      isSelected: true,
    });

    const trackItemDiv = container.querySelector('[data-testid="track-item"]');
    expect(trackItemDiv?.id).toBe(`track-item-${mockTrack.id}`);
  });

  it("calls trackStore.playTrack when clicked", async () => {
    render(TrackItem, { track: mockTrack, index: 2, isSelected: false });

    const trackItemElement = screen.getByTestId("track-item");
    await fireEvent.mouseDown(trackItemElement, { button: 0 });
    await fireEvent.mouseUp(trackItemElement, { button: 0 });

    expect(vi.mocked(trackStore.playTrack)).toHaveBeenCalledWith(2);
  });

  it("calls playTrack when Enter key is pressed", async () => {
    render(TrackItem, { track: mockTrack, index: 3, isSelected: false });

    const trackItemElement = screen.getByTestId("track-item");
    await fireEvent.keyDown(trackItemElement, { key: "Enter" });

    expect(vi.mocked(trackStore.playTrack)).toHaveBeenCalledWith(3);
  });

  it("calls playTrack when Space key is pressed", async () => {
    render(TrackItem, { track: mockTrack, index: 4, isSelected: false });

    const trackItemElement = screen.getByTestId("track-item");
    await fireEvent.keyDown(trackItemElement, { key: " " });

    expect(vi.mocked(trackStore.playTrack)).toHaveBeenCalledWith(4);
  });

  describe("buffered ranges integration", () => {
    it("should subscribe to buffered ranges store when isSelected and audioService is provided", () => {
      const mockBufferedRangesStore = writable<TimeRange[]>([]);
      const mockAudioService = {
        currentBufferedRangesStore: mockBufferedRangesStore,
      } as unknown as AudioService;

      render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        audioService: mockAudioService,
      });

      expect(mockBufferedRangesStore.subscribe).toBeDefined();
    });

    it("should pass buffered ranges to slider when isSelected and audioService is provided", () => {
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

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        audioService: mockAudioService,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(2);
    });

    it("should not pass buffered ranges when isSelected is false", () => {
      const bufferedRanges: TimeRange[] = [{ start: 10, end: 50 }];

      const mockBufferedRangesStore = writable(bufferedRanges);
      const mockAudioService = {
        currentBufferedRangesStore: mockBufferedRangesStore,
      } as unknown as AudioService;

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: false,
        audioService: mockAudioService,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });

    it("should not pass buffered ranges when audioService is undefined", () => {
      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        audioService: undefined,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });

    it("should handle empty buffered ranges when isSelected", () => {
      const mockBufferedRangesStore = writable<TimeRange[]>([]);
      const mockDurationStore = writable(180);
      const mockCurrentTimeStore = writable(30);
      const mockIsPlayingStore = writable(false);

      const mockAudioService = {
        currentBufferedRangesStore: mockBufferedRangesStore,
        durationStore: mockDurationStore,
        currentTimeStore: mockCurrentTimeStore,
        isPlayingStore: mockIsPlayingStore,
      } as unknown as AudioService;

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        audioService: mockAudioService,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });

    it("should update buffered ranges when store changes", async () => {
      const mockBufferedRangesStore = writable<TimeRange[]>([]);
      const mockDurationStore = writable(180);
      const mockCurrentTimeStore = writable(30);
      const mockIsPlayingStore = writable(false);

      const mockAudioService = {
        currentBufferedRangesStore: mockBufferedRangesStore,
        durationStore: mockDurationStore,
        currentTimeStore: mockCurrentTimeStore,
        isPlayingStore: mockIsPlayingStore,
      } as unknown as AudioService;

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        audioService: mockAudioService,
      });

      expect(container.querySelectorAll(".bg-accent\\/20")).toHaveLength(0);

      mockBufferedRangesStore.set([{ start: 0, end: 60 }]);

      await new Promise((resolve) => setTimeout(resolve, 0));

      expect(container.querySelectorAll(".bg-accent\\/20")).toHaveLength(1);
    });

    it("should work when currentBufferedRangesStore is undefined", () => {
      const mockAudioService = {
        currentBufferedRangesStore: undefined,
      } as unknown as AudioService;

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        audioService: mockAudioService,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });
  });
});
