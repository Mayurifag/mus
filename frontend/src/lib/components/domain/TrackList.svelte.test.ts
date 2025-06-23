import { vi } from "vitest";

// Mock browser environment
const mockScrollIntoView = vi.fn();
vi.mock("$app/environment", () => ({
  browser: true,
}));

// Mock IntersectionObserver
const mockIntersectionObserver = vi.fn();
mockIntersectionObserver.prototype.observe = vi.fn();
mockIntersectionObserver.prototype.disconnect = vi.fn();
globalThis.IntersectionObserver = mockIntersectionObserver;

// Mock tick function
vi.mock("svelte", async () => {
  const actual = await vi.importActual("svelte");
  return {
    ...actual,
    tick: vi.fn().mockResolvedValue(undefined),
  };
});

// Create a mock document.getElementById
const originalGetElementById = document.getElementById.bind(document);
document.getElementById = vi.fn().mockImplementation((id) => {
  if (id === "track-item-1") {
    return {
      scrollIntoView: mockScrollIntoView,
    };
  }
  return originalGetElementById(id);
});

// Mock the trackStore
const mockTrackStoreData: { currentTrackIndex: number; tracks: Track[] } = {
  currentTrackIndex: 0,
  tracks: [],
};
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    subscribe: vi.fn().mockImplementation((callback) => {
      callback(mockTrackStoreData);
      return () => {};
    }),
  },
}));

// Mock the TrackItem component
vi.mock("./TrackItem.svelte", () => ({
  default: vi.fn().mockImplementation(() => ({
    $$typeof: Symbol.for("svelte.component"),
  })),
}));

import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";
import type { Track } from "$lib/types";
import "@testing-library/jest-dom/vitest";
import TrackList from "./TrackList.svelte";
import TrackItem from "./TrackItem.svelte";

describe("TrackList component", () => {
  let mockTracks: Track[];

  beforeEach(() => {
    mockTracks = [
      {
        id: 1,
        title: "Song 1",
        artist: "Artist 1",
        duration: 180,
        has_cover: true,
        cover_small_url: "/api/v1/tracks/1/covers/small.webp",
        cover_original_url: "/api/v1/tracks/1/covers/original.webp",
      },
      {
        id: 2,
        title: "Song 2",
        artist: "Artist 2",
        duration: 240,
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
      },
    ];

    // Clear mocks
    vi.mocked(TrackItem).mockClear();
    vi.mocked(document.getElementById).mockClear();
    mockScrollIntoView.mockClear();
    mockIntersectionObserver.mockClear();
    mockIntersectionObserver.prototype.observe.mockClear();
    mockIntersectionObserver.prototype.disconnect.mockClear();
  });

  it("renders the track list element when tracks are provided", () => {
    mockTrackStoreData.tracks = mockTracks;
    render(TrackList);

    const trackListElement = screen.getByTestId("track-list");
    expect(trackListElement).toBeInTheDocument();
  });

  it("renders empty state messages when no tracks are available", () => {
    mockTrackStoreData.tracks = [];
    render(TrackList);

    expect(screen.getByText("No tracks available")).toBeInTheDocument();
  });

  it("renders the right number of TrackItems when tracks are provided", () => {
    mockTrackStoreData.tracks = mockTracks;
    render(TrackList);

    // Check the track container exists
    const trackList = screen.getByTestId("track-list");
    const trackContainer = trackList.querySelector(".space-y-1");
    expect(trackContainer).not.toBeNull();

    // Verify the mock component was called the expected number of times
    expect(vi.mocked(TrackItem)).toHaveBeenCalledTimes(mockTracks.length);
  });
});
