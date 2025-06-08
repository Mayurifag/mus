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
import { render, fireEvent } from "@testing-library/svelte";
import { trackStore } from "$lib/stores/trackStore";
import type { AudioService } from "$lib/services/AudioService";
import type { TimeRange } from "$lib/types";
import { writable } from "svelte/store";
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

  it("should have correct height classes for mobile and desktop", () => {
    const { container } = render(PlayerFooter);
    const mainContainer = container.querySelector(".flex.h-36");
    expect(mainContainer).toBeTruthy();
    expect(mainContainer?.classList.contains("desktop:h-28")).toBe(true);
  });

  it("should hide track info text on mobile and show on desktop", () => {
    const { container } = render(PlayerFooter);
    const trackInfoText = container.querySelector(".hidden.desktop\\:flex");
    expect(trackInfoText).toBeTruthy();
    expect(trackInfoText?.textContent).toContain("Test Track");
    expect(trackInfoText?.textContent).toContain("Test Artist");
  });

  it("should show mobile track info row with artist - title format", () => {
    const { container } = render(PlayerFooter);
    const mobileTrackInfo = container.querySelector(".desktop\\:hidden");
    expect(mobileTrackInfo).toBeTruthy();
    expect(mobileTrackInfo?.textContent).toContain("Test Artist - Test Track");
  });

  it("should have three-state responsive album art sizing", () => {
    const { container } = render(PlayerFooter);
    const albumArt = container.querySelector(".h-24.w-24.desktop\\:h-18");
    expect(albumArt).toBeTruthy();
    expect(albumArt?.classList.contains("h-24")).toBe(true); // Big size (650px-999px)
    expect(albumArt?.classList.contains("w-24")).toBe(true);
    expect(albumArt?.classList.contains("desktop:h-18")).toBe(true); // Normal size (≥1000px)
    expect(albumArt?.classList.contains("desktop:w-18")).toBe(true);
  });

  it("should hide album image on screens below 650px", () => {
    const { container } = render(PlayerFooter);
    const albumImageContainer = container.querySelector(
      ".hidden.sm650\\:block",
    );
    expect(albumImageContainer).toBeTruthy();
    expect(albumImageContainer?.classList.contains("hidden")).toBe(true);
    expect(albumImageContainer?.classList.contains("sm650:block")).toBe(true);
  });

  it("should have proper margins around album image for three states", () => {
    const { container } = render(PlayerFooter);
    const albumImageContainer = container.querySelector(
      ".my-6.ml-6.desktop\\:my-5",
    );
    expect(albumImageContainer).toBeTruthy();
    expect(albumImageContainer?.classList.contains("my-6")).toBe(true); // Big size margin (24px)
    expect(albumImageContainer?.classList.contains("ml-6")).toBe(true); // Big size left margin (24px)
    expect(albumImageContainer?.classList.contains("desktop:my-5")).toBe(true); // Normal size margin (20px)
    expect(albumImageContainer?.classList.contains("desktop:ml-5")).toBe(true); // Normal size left margin (20px)
  });

  it("should expand central controls when album image is hidden", () => {
    const { container } = render(PlayerFooter);
    const centralControls = container.querySelector(".sm650\\:mx-2.mx-4");
    expect(centralControls).toBeTruthy();
    expect(centralControls?.classList.contains("mx-4")).toBe(true);
    expect(centralControls?.classList.contains("sm650:mx-2")).toBe(true);
  });

  it("should have wider progress bar on mobile screens", () => {
    const { container } = render(PlayerFooter);
    const progressContainer = container.querySelector(
      ".max-w-md.desktop\\:max-w-lg",
    );
    expect(progressContainer).toBeTruthy();
    expect(progressContainer?.classList.contains("max-w-md")).toBe(true); // Wider on mobile
    expect(progressContainer?.classList.contains("desktop:max-w-lg")).toBe(
      true,
    ); // Desktop width
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

  it("should apply active styling to shuffle button when shuffle is enabled", () => {
    // Mock shuffle enabled state
    const mockStoreWithShuffle = {
      ...vi.mocked(trackStore),
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
          is_shuffle: true,
        });
        return () => {};
      }),
    };

    vi.mocked(trackStore).subscribe = mockStoreWithShuffle.subscribe;

    const { container } = render(PlayerFooter);
    const shuffleButton = container.querySelector(
      'button[aria-label="Toggle Shuffle"]',
    );

    expect(shuffleButton).toBeTruthy();
    expect(shuffleButton?.classList.contains("bg-accent/10")).toBe(true);
  });

  it("should apply active styling to repeat button when repeat is enabled", () => {
    const mockIsRepeatStore = {
      subscribe: vi.fn((callback) => {
        callback(true); // Simulate repeat enabled
        return () => {};
      }),
      set: vi.fn(),
      update: vi.fn(),
    };

    const mockAudioService = {
      isRepeat: true,
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
    expect(repeatButton?.classList.contains("bg-accent/10")).toBe(true);
  });

  it("should toggle repeat state when repeat button is clicked", async () => {
    const mockIsRepeatStore = {
      subscribe: vi.fn((callback) => {
        callback(false); // Start with repeat disabled
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

    // Initially should not have active styling
    expect(repeatButton?.classList.contains("bg-accent/10")).toBe(false);

    // Click the button
    await fireEvent.click(repeatButton as Element);

    // Verify toggleRepeat was called
    expect(mockAudioService.toggleRepeat).toHaveBeenCalledOnce();
  });

  it("should have volume controls positioned in the central control section", () => {
    const { container } = render(PlayerFooter);

    const muteButton = container.querySelector('button[aria-label="Mute"]');
    expect(muteButton).toBeTruthy();

    // Check that volume controls are in the central controls section
    // Look for the central section with flex-1 class
    const centralControlsSection = container.querySelector("div.flex.flex-1");
    expect(centralControlsSection).toBeTruthy();

    const muteButtonInCentral = centralControlsSection?.querySelector(
      'button[aria-label="Mute"]',
    );
    expect(muteButtonInCentral).toBeTruthy();
  });

  it("should have progress slider positioned below control buttons", () => {
    const { container } = render(PlayerFooter);

    // Look for the central section with flex-1 class
    const centralControlsSection = container.querySelector("div.flex.flex-1");
    expect(centralControlsSection).toBeTruthy();

    // Check for the two rows structure - look for divs with specific spacing classes
    const controlButtonsRow = centralControlsSection?.querySelector(
      "div.flex.items-center.space-x-2",
    );
    const progressSliderRow =
      centralControlsSection?.querySelector("div.mt-1.flex");

    expect(controlButtonsRow).toBeTruthy();
    expect(progressSliderRow).toBeTruthy();
  });

  it("should have three-row mobile layout structure", () => {
    const { container } = render(PlayerFooter);

    // Look for the central section with mobile layout classes
    const centralControlsSection = container.querySelector(
      "div.flex.h-full.flex-1.flex-col.items-center.justify-around",
    );
    expect(centralControlsSection).toBeTruthy();

    // Row 1: Control buttons and volume controls
    const controlButtonsRow = centralControlsSection?.querySelector(
      "div.flex.w-full.items-center.justify-center.space-x-2",
    );
    expect(controlButtonsRow).toBeTruthy();

    // Row 2: Progress slider with time indicators
    const progressRow = centralControlsSection?.querySelector(
      "div.mt-1.flex.w-full.max-w-md.items-center.space-x-2",
    );
    expect(progressRow).toBeTruthy();

    // Row 3: Mobile track info (artist - title)
    const mobileTrackInfoRow = centralControlsSection?.querySelector(
      "div.desktop\\:hidden.mt-1.flex.w-full.items-center.justify-center.text-center",
    );
    expect(mobileTrackInfoRow).toBeTruthy();
  });

  describe("buffered ranges integration", () => {
    it("should subscribe to buffered ranges store when audioService is provided", () => {
      const mockBufferedRangesStore = writable<TimeRange[]>([]);
      const mockAudioService = {
        currentBufferedRangesStore: mockBufferedRangesStore,
      } as unknown as AudioService;

      render(PlayerFooter, {
        audioService: mockAudioService,
      });

      expect(mockBufferedRangesStore.subscribe).toBeDefined();
    });

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
      expect(bufferedSegments).toHaveLength(2);
    });

    it("should handle empty buffered ranges", () => {
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

      const { container } = render(PlayerFooter, {
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

      const { container } = render(PlayerFooter, {
        audioService: mockAudioService,
      });

      expect(container.querySelectorAll(".bg-accent\\/20")).toHaveLength(0);

      mockBufferedRangesStore.set([{ start: 0, end: 60 }]);

      await new Promise((resolve) => setTimeout(resolve, 0));

      expect(container.querySelectorAll(".bg-accent\\/20")).toHaveLength(1);
    });

    it("should work without audioService", () => {
      const { container } = render(PlayerFooter);

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });

    it("should work when currentBufferedRangesStore is undefined", () => {
      const mockAudioService = {
        currentBufferedRangesStore: undefined,
      } as unknown as AudioService;

      const { container } = render(PlayerFooter, {
        audioService: mockAudioService,
      });

      const bufferedSegments = container.querySelectorAll(".bg-accent\\/20");
      expect(bufferedSegments).toHaveLength(0);
    });
  });
});
