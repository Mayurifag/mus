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
import { render, fireEvent } from "@testing-library/svelte";
import type { AudioService } from "$lib/services/AudioService";
import type { TimeRange } from "$lib/types";
import { writable } from "svelte/store";
import PlayerFooter from "./PlayerFooter.svelte";

describe("PlayerFooter component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render without errors", () => {
    const { container } = render(PlayerFooter);
    expect(container).toBeTruthy();
    expect(container.textContent).toContain("Test Track");
    expect(container.textContent).toContain("Test Artist");
  });

  it("should have correct height classes for mobile and desktop", () => {
    const { container } = render(PlayerFooter);
    const rootContainer = container.querySelector(".fixed");
    expect(rootContainer).toBeTruthy();
    expect(
      rootContainer?.classList.contains("h-[var(--footer-height-mobile)]"),
    ).toBe(true);
    expect(
      rootContainer?.classList.contains(
        "sm700:h-[var(--footer-height-desktop)]",
      ),
    ).toBe(true);
  });

  it("should have mobile container visible on small screens and hidden on large screens", () => {
    const { container } = render(PlayerFooter);
    const mobileContainer = container.querySelector(".sm700\\:hidden");
    expect(mobileContainer).toBeTruthy();
    expect(mobileContainer?.classList.contains("flex")).toBe(true);
    expect(mobileContainer?.classList.contains("flex-col")).toBe(true);
    expect(mobileContainer?.classList.contains("sm700:hidden")).toBe(true);
  });

  it("should have desktop container hidden on small screens and visible on large screens", () => {
    const { container } = render(PlayerFooter);
    const desktopContainer = container.querySelector(".sm700\\:flex.hidden");
    expect(desktopContainer).toBeTruthy();
    expect(desktopContainer?.classList.contains("hidden")).toBe(true);
    expect(desktopContainer?.classList.contains("sm700:flex")).toBe(true);
  });

  it("should have four distinct rows in mobile layout", () => {
    const { container } = render(PlayerFooter);
    const mobileContainer = container.querySelector(".sm700\\:hidden");
    expect(mobileContainer).toBeTruthy();

    const rows = mobileContainer?.children;
    expect(rows?.length).toBe(4);

    // Row 1: Controls
    const controlsRow = rows?.[0];
    expect(controlsRow?.classList.contains("justify-center")).toBe(true);

    // Row 2: Volume & Menu
    const volumeRow = rows?.[1];
    expect(volumeRow?.classList.contains("gap-3")).toBe(true);

    // Row 3: Progress
    const progressRow = rows?.[2];
    expect(progressRow?.classList.contains("gap-3")).toBe(true);

    // Row 4: Track Info
    const trackInfoRow = rows?.[3];
    expect(trackInfoRow?.classList.contains("justify-center")).toBe(true);
  });

  it("should show mobile track info in row 4 with artist - title format", () => {
    const { container } = render(PlayerFooter);
    const mobileContainer = container.querySelector(".sm700\\:hidden");
    const trackInfoRow = mobileContainer?.children[3];
    expect(trackInfoRow?.textContent).toContain("Test Artist - Test Track");
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
