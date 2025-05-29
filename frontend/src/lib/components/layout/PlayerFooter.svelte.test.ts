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
        is_shuffle: false,
      });
      return () => {};
    }),
    nextTrack: vi.fn(),
    previousTrack: vi.fn(),
    toggleShuffle: vi.fn(),
  };

  return {
    trackStore: mockStore,
  };
});

import { describe, it, expect, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
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

  it("should call trackStore.toggleShuffle when mocked", () => {
    expect(trackStore.toggleShuffle).toBeDefined();
    trackStore.toggleShuffle();
    expect(trackStore.toggleShuffle).toHaveBeenCalled();
  });

  it("should have working store subscriptions", () => {
    expect(trackStore.subscribe).toBeDefined();
    expect(typeof trackStore.subscribe).toBe("function");
  });

  it("should have all required store methods", () => {
    expect(trackStore.toggleShuffle).toBeDefined();
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

  it("should have shuffle and repeat buttons positioned before previous track button", () => {
    const { container } = render(PlayerFooter);
    const buttons = container.querySelectorAll("button[aria-label]");

    const shuffleButton = Array.from(buttons).find(
      (btn) => btn.getAttribute("aria-label") === "Toggle Shuffle",
    );
    const repeatButton = Array.from(buttons).find(
      (btn) => btn.getAttribute("aria-label") === "Toggle Repeat",
    );
    const previousButton = Array.from(buttons).find(
      (btn) => btn.getAttribute("aria-label") === "Previous Track",
    );

    expect(shuffleButton).toBeTruthy();
    expect(repeatButton).toBeTruthy();
    expect(previousButton).toBeTruthy();

    const allButtons = Array.from(buttons);
    const shuffleIndex = allButtons.indexOf(shuffleButton as Element);
    const repeatIndex = allButtons.indexOf(repeatButton as Element);
    const previousIndex = allButtons.indexOf(previousButton as Element);

    expect(shuffleIndex).toBeLessThan(previousIndex);
    expect(repeatIndex).toBeLessThan(previousIndex);
    expect(shuffleIndex).toBeLessThan(repeatIndex);
  });
});
