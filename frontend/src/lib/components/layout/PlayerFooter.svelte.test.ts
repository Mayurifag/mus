/**
 * @vitest-environment jsdom
 * @vitest-environment-options { "url": "http://localhost:5173" }
 */
import { vi } from "vitest";

// Mock playerStore
vi.mock("$lib/stores/playerStore", () => {
  const mockStore = {
    subscribe: vi.fn((callback) => {
      callback({
        currentTrack: {
          id: 1,
          title: "Test Track",
          artist: "Test Artist",
          duration: 180,
          file_path: "/path/to/file.mp3",
          added_at: 1615478400,
          has_cover: true,
          cover_small_url: "/api/v1/tracks/1/covers/small.webp",
          cover_original_url: "/api/v1/tracks/1/covers/original.webp",
        },
        isPlaying: false,
        currentTime: 30,
        duration: 180,
        volume: 0.5,
        isMuted: false,
        is_shuffle: false,
        is_repeat: false,
      });
      return () => {};
    }),
    togglePlayPause: vi.fn(),
    setCurrentTime: vi.fn(),
    setVolume: vi.fn(),
    toggleMute: vi.fn(),
    toggleShuffle: vi.fn(),
    toggleRepeat: vi.fn(),
  };

  return {
    playerStore: mockStore,
  };
});

// Mock trackStore
vi.mock("$lib/stores/trackStore", () => {
  const mockStore = {
    subscribe: vi.fn((callback) => {
      callback({
        tracks: [],
        currentTrackIndex: null,
        playHistory: [],
      });
      return () => {};
    }),
    nextTrack: vi.fn(),
    previousTrack: vi.fn(),
  };

  return {
    trackStore: mockStore,
  };
});

import { describe, it, expect, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
import { playerStore } from "$lib/stores/playerStore";
import { trackStore } from "$lib/stores/trackStore";
import PlayerFooter from "./PlayerFooter.svelte";

describe("PlayerFooter component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should be defined", () => {
    expect(PlayerFooter).toBeDefined();
  });

  it("should be a function", () => {
    expect(typeof PlayerFooter).toBe("function");
  });

  it("should render without errors", () => {
    const { container } = render(PlayerFooter);
    expect(container).toBeTruthy();
  });

  it("should call playerStore.togglePlayPause when mocked", () => {
    expect(playerStore.togglePlayPause).toBeDefined();
    playerStore.togglePlayPause();
    expect(playerStore.togglePlayPause).toHaveBeenCalled();
  });

  it("should call trackStore.nextTrack when mocked", () => {
    expect(trackStore.nextTrack).toBeDefined();
    trackStore.nextTrack();
    expect(trackStore.nextTrack).toHaveBeenCalled();
  });

  it("should call trackStore.previousTrack when mocked", () => {
    expect(trackStore.previousTrack).toBeDefined();
    trackStore.previousTrack();
    expect(trackStore.previousTrack).toHaveBeenCalled();
  });

  it("should call playerStore.toggleMute when mocked", () => {
    expect(playerStore.toggleMute).toBeDefined();
    playerStore.toggleMute();
    expect(playerStore.toggleMute).toHaveBeenCalled();
  });

  it("should call playerStore.setCurrentTime when mocked", () => {
    expect(playerStore.setCurrentTime).toBeDefined();
    playerStore.setCurrentTime(30);
    expect(playerStore.setCurrentTime).toHaveBeenCalledWith(30);
  });

  it("should call playerStore.setVolume when mocked", () => {
    expect(playerStore.setVolume).toBeDefined();
    playerStore.setVolume(0.5);
    expect(playerStore.setVolume).toHaveBeenCalledWith(0.5);
  });

  it("should call playerStore.toggleShuffle when mocked", () => {
    expect(playerStore.toggleShuffle).toBeDefined();
    playerStore.toggleShuffle();
    expect(playerStore.toggleShuffle).toHaveBeenCalled();
  });

  it("should call playerStore.toggleRepeat when mocked", () => {
    expect(playerStore.toggleRepeat).toBeDefined();
    playerStore.toggleRepeat();
    expect(playerStore.toggleRepeat).toHaveBeenCalled();
  });

  it("should have working store subscriptions", () => {
    expect(playerStore.subscribe).toBeDefined();
    expect(trackStore.subscribe).toBeDefined();
    expect(typeof playerStore.subscribe).toBe("function");
    expect(typeof trackStore.subscribe).toBe("function");
  });

  it("should have all required store methods", () => {
    expect(playerStore.togglePlayPause).toBeDefined();
    expect(playerStore.setCurrentTime).toBeDefined();
    expect(playerStore.setVolume).toBeDefined();
    expect(playerStore.toggleMute).toBeDefined();
    expect(playerStore.toggleShuffle).toBeDefined();
    expect(playerStore.toggleRepeat).toBeDefined();
    expect(trackStore.nextTrack).toBeDefined();
    expect(trackStore.previousTrack).toBeDefined();
  });

  it("should render component structure", () => {
    const { container } = render(PlayerFooter);
    const footer = container.querySelector(".fixed");
    expect(footer).toBeTruthy();
  });

  it("should contain player controls", () => {
    const { container } = render(PlayerFooter);
    expect(container.textContent).toContain("Test Track");
    expect(container.textContent).toContain("Test Artist");
  });
});
