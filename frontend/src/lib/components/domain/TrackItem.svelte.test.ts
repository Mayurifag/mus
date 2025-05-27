import { vi } from "vitest";

// Mock the trackStore
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    playTrack: vi.fn(),
  },
}));

// Mock the playerStore
vi.mock("$lib/stores/playerStore", () => ({
  playerStore: {
    pause: vi.fn(),
    play: vi.fn(),
    setCurrentTime: vi.fn(),
    subscribe: vi.fn().mockImplementation((callback) => {
      callback({
        isPlaying: true,
        currentTime: 60, // 1 minute
        duration: 180, // 3 minutes
      });
      return () => {};
    }),
  },
}));

// Mock date-fns formatDistanceToNow
vi.mock("date-fns", () => ({
  formatDistanceToNow: vi.fn().mockReturnValue("about 1 hour ago"),
}));

import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/svelte";
import type { Track } from "$lib/types";
import "@testing-library/jest-dom/vitest";
import TrackItem from "./TrackItem.svelte";
import { trackStore } from "$lib/stores/trackStore";
import { playerStore } from "$lib/stores/playerStore";

describe("TrackItem component", () => {
  let mockTrack: Track;

  beforeEach(() => {
    mockTrack = {
      id: 1,
      title: "Test Song",
      artist: "Test Artist",
      duration: 180,
      file_path: "/path/to/song.mp3",
      added_at: Math.floor(Date.now() / 1000) - 3600, // 1 hour ago
      has_cover: true,
      cover_small_url: "/api/v1/tracks/1/covers/small.webp",
      cover_original_url: "/api/v1/tracks/1/covers/original.webp",
    };

    // Clear the mocks
    vi.mocked(trackStore.playTrack).mockClear();
    vi.mocked(playerStore.pause).mockClear();
    vi.mocked(playerStore.play).mockClear();
    vi.mocked(playerStore.setCurrentTime).mockClear();
  });

  it("renders track details correctly", () => {
    render(TrackItem, { track: mockTrack, index: 0, isSelected: false });

    expect(screen.getByText("Test Song")).toBeInTheDocument();
    expect(screen.getByText("Test Artist")).toBeInTheDocument();
    expect(screen.getByText("3:00")).toBeInTheDocument(); // 180 seconds formatted
    expect(screen.getByAltText("Album art for Test Song")).toBeInTheDocument();
    expect(screen.getByText("about 1 hour ago")).toBeInTheDocument();
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

    expect(screen.getByText("No Cover")).toBeInTheDocument();
  });

  it("applies selected styles when isSelected is true", () => {
    const { container } = render(TrackItem, {
      track: mockTrack,
      index: 0,
      isSelected: true,
    });
    const trackItemDiv = container.querySelector('[data-testid="track-item"]');

    expect(trackItemDiv?.classList.contains("bg-muted")).toBe(true);
  });

  it("renders progress slider when track is selected", () => {
    render(TrackItem, { track: mockTrack, index: 0, isSelected: true });

    const progressSlider = screen.getByTestId("track-progress-slider");
    expect(progressSlider).toBeInTheDocument();

    // Check that it has the correct class for styling
    expect(progressSlider.classList.contains("track-progress-slider")).toBe(
      true,
    );
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
    await fireEvent.click(trackItemElement);

    expect(vi.mocked(trackStore.playTrack)).toHaveBeenCalledWith(2);
  });

  it("calls playerStore.pause when clicked if already playing", async () => {
    render(TrackItem, { track: mockTrack, index: 2, isSelected: true });

    const trackItemElement = screen.getByTestId("track-item");
    await fireEvent.click(trackItemElement);

    expect(vi.mocked(playerStore.pause)).toHaveBeenCalled();
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

  it("calls playerStore.play when selected track is paused and clicked", async () => {
    // Mock playerStore to return paused state
    vi.mocked(playerStore.subscribe).mockImplementation((callback) => {
      callback({
        currentTrack: mockTrack,
        isPlaying: false,
        currentTime: 60,
        duration: 180,
        volume: 1.0,
        isMuted: false,
        is_shuffle: false,
        is_repeat: false,
      });
      return () => {};
    });

    render(TrackItem, { track: mockTrack, index: 2, isSelected: true });

    const trackItemElement = screen.getByTestId("track-item");
    await fireEvent.click(trackItemElement);

    expect(vi.mocked(playerStore.play)).toHaveBeenCalled();
  });
});
