import { vi } from "vitest";

const mockTrackStoreState = vi.hoisted(() => ({
  selectedArtist: null as string | null,
  tracks: [] as { artist: string }[],
}));

const mockTrackStore = vi.hoisted(() => ({
  subscribe: vi.fn((callback: (value: typeof mockTrackStoreState) => void) => {
    callback(mockTrackStoreState);
    return () => {};
  }),
  playTrack: vi.fn(),
  setArtistFilter: vi.fn(),
}));

const mockArtistCountsStore = vi.hoisted(() => ({
  subscribe: vi.fn((callback: (value: Record<string, number>) => void) => {
    const counts = new Map<string, number>();
    for (const track of mockTrackStoreState.tracks) {
      const artists = track.artist
        .split(/[;,]/)
        .map((value) => value.trim())
        .filter(Boolean);
      for (const artist of new Set(artists)) {
        counts.set(artist, (counts.get(artist) ?? 0) + 1);
      }
    }
    callback(Object.fromEntries(counts));
    return () => {};
  }),
}));

vi.mock("$lib/stores/trackStore", () => ({
  artistCountsStore: mockArtistCountsStore,
  trackStore: mockTrackStore,
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
import "@testing-library/jest-dom/vitest";
import TrackItem from "./TrackItem.svelte";
import { trackStore } from "$lib/stores/trackStore";

describe("TrackItem component", () => {
  let mockTrack: Track;

  beforeEach(() => {
    mockTrack = {
      id: "1",
      title: "Test Song",
      artist: "Test Artist",
      duration: 180,
      filename: "test-track.mp3",
      has_cover: true,
      cover_small_url: "/api/v1/tracks/1/covers/small.webp",
      cover_original_url: "/api/v1/tracks/1/covers/original.webp",
      hls_url: "/api/v1/tracks/1/hls/1640995200/index.m3u8",
      updated_at: 1640995200,
    };

    // Clear the mocks
    vi.mocked(trackStore.playTrack).mockClear();
    vi.mocked(trackStore.setArtistFilter).mockClear();
    mockTrackStoreState.selectedArtist = null;
    mockTrackStoreState.tracks = [mockTrack, { ...mockTrack, id: "2" }];
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

  it("renders progress slider when track is selected", () => {
    render(TrackItem, { track: mockTrack, index: 0, isSelected: true });

    const progressSlider = screen.getByTestId("track-progress-slider");
    expect(progressSlider).toBeInTheDocument();
  });

  it("does not toggle playback when selected track slider is clicked", async () => {
    const audioService = {
      play: vi.fn(),
      pause: vi.fn(),
      startSeeking: vi.fn(),
      seek: vi.fn(),
      endSeeking: vi.fn(),
    } as unknown as AudioService;

    render(TrackItem, {
      track: mockTrack,
      index: 0,
      isSelected: true,
      isPlaying: false,
      audioService,
      duration: 180,
      currentTime: 30,
    });

    const progressSlider = screen.getByTestId("track-progress-slider");
    await fireEvent.mouseDown(progressSlider, { button: 0 });
    await fireEvent.mouseUp(progressSlider, { button: 0 });

    expect(audioService.play).not.toHaveBeenCalled();
    expect(audioService.pause).not.toHaveBeenCalled();
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

  it("styles right-version and ai-cover rows from title markers", () => {
    const { container } = render(TrackItem, {
      track: { ...mockTrack, title: "Test Song (Right version)" },
      index: 0,
      isSelected: false,
    });

    expect(screen.getByTestId("track-item")).toHaveClass(
      "border-fuchsia-400/70",
    );

    container.remove();

    render(TrackItem, {
      track: { ...mockTrack, title: "Test Song (AI cover)" },
      index: 0,
      isSelected: false,
    });

    expect(screen.getByTestId("track-item")).toHaveClass("border-cyan-400/70");
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

  it("filters by artist when artist is clicked", async () => {
    render(TrackItem, { track: mockTrack, index: 0, isSelected: false });

    const artistButton = screen.getByRole("button", {
      name: "Show Test Artist songs",
    });
    await fireEvent.mouseDown(artistButton, { button: 0 });
    await fireEvent.mouseUp(artistButton, { button: 0 });
    await fireEvent.click(artistButton);

    expect(vi.mocked(trackStore.setArtistFilter)).toHaveBeenCalledWith(
      "Test Artist",
    );
    expect(vi.mocked(trackStore.playTrack)).not.toHaveBeenCalled();
  });

  it("renders duplicate artists once", () => {
    mockTrack.artist = "Test Artist; Test Artist";
    mockTrackStoreState.tracks = [mockTrack, { ...mockTrack, id: "2" }];

    render(TrackItem, { track: mockTrack, index: 0, isSelected: false });

    expect(
      screen.getAllByRole("button", { name: "Show Test Artist songs" }),
    ).toHaveLength(1);
  });

  it("renders spaces after artist separators", () => {
    mockTrack.artist = "Artist A; Artist B";
    mockTrackStoreState.tracks = [mockTrack, { ...mockTrack, id: "2" }];

    render(TrackItem, { track: mockTrack, index: 0, isSelected: false });

    expect(screen.getByTestId("track-item")).toHaveTextContent(
      "Artist A, Artist B",
    );
  });

  it("plays the row when selected artist text is clicked", async () => {
    mockTrackStoreState.selectedArtist = "Test Artist";
    render(TrackItem, { track: mockTrack, index: 0, isSelected: false });

    expect(
      screen.queryByRole("button", {
        name: "Show Test Artist songs",
      }),
    ).not.toBeInTheDocument();

    const artistText = screen.getByText("Test Artist");
    await fireEvent.mouseDown(artistText, { button: 0 });
    await fireEvent.mouseUp(artistText, { button: 0 });

    expect(vi.mocked(trackStore.setArtistFilter)).not.toHaveBeenCalled();
    expect(vi.mocked(trackStore.playTrack)).toHaveBeenCalledWith(0);
  });

  it("plays the row when a one-song artist is clicked", async () => {
    mockTrackStoreState.tracks = [mockTrack];
    render(TrackItem, { track: mockTrack, index: 0, isSelected: false });

    expect(
      screen.queryByRole("button", {
        name: "Show Test Artist songs",
      }),
    ).not.toBeInTheDocument();

    const artistText = screen.getByText("Test Artist");
    await fireEvent.mouseDown(artistText, { button: 0 });
    await fireEvent.mouseUp(artistText, { button: 0 });

    expect(vi.mocked(trackStore.setArtistFilter)).not.toHaveBeenCalled();
    expect(vi.mocked(trackStore.playTrack)).toHaveBeenCalledWith(0);
  });

  describe("buffered ranges integration", () => {
    it("should render buffered ranges when provided as props", () => {
      const bufferedRanges: TimeRange[] = [{ start: 10, end: 50 }];

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        bufferedRanges,
        duration: 180,
        currentTime: 30,
        isPlaying: false,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(1);
    });

    it("should pass buffered ranges to slider when isSelected and bufferedRanges prop is provided", () => {
      const bufferedRanges: TimeRange[] = [
        { start: 10, end: 50 },
        { start: 80, end: 120 },
      ];

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        bufferedRanges,
        duration: 180,
        currentTime: 30,
        isPlaying: false,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(2);
    });

    it("should not pass buffered ranges when isSelected is false", () => {
      const bufferedRanges: TimeRange[] = [{ start: 10, end: 50 }];

      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: false,
        bufferedRanges,
        duration: 180,
        currentTime: 30,
        isPlaying: false,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });

    it("should not show buffered ranges when bufferedRanges prop is undefined", () => {
      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        audioService: undefined,
        duration: 180,
        currentTime: 30,
        isPlaying: false,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });

    it("should handle empty buffered ranges when isSelected", () => {
      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        bufferedRanges: [],
        duration: 180,
        currentTime: 30,
        isPlaying: false,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });

    it("should render buffered ranges correctly", () => {
      // Test with buffered ranges provided
      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        bufferedRanges: [{ start: 0, end: 60 }],
        duration: 180,
        currentTime: 30,
        isPlaying: false,
      });

      expect(container.querySelectorAll(".bg-accent\\/20")).toHaveLength(1);
    });

    it("should work when bufferedRanges prop is undefined", () => {
      const { container } = render(TrackItem, {
        track: mockTrack,
        index: 0,
        isSelected: true,
        duration: 180,
        currentTime: 30,
        isPlaying: false,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });
  });
});
