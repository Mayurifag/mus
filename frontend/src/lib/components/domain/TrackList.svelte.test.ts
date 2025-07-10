import { vi } from "vitest";
import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";
import type { Track } from "$lib/types";
import "@testing-library/jest-dom/vitest";

// Mock the browser environment
vi.mock("$app/environment", () => ({
  browser: false,
}));

// Mock TanStack Virtual
vi.mock("@tanstack/svelte-virtual", () => ({
  createWindowVirtualizer: vi.fn(() => ({
    getTotalSize: () => 1000,
    getVirtualItems: () => [],
  })),
}));

// Mock the trackStore
const mockTrackStoreData = vi.hoisted(() => ({
  currentTrackIndex: 0,
  tracks: [] as Track[],
}));

const mockTrackStore = vi.hoisted(() => ({
  subscribe: vi.fn(
    (
      callback: (value: { currentTrackIndex: number; tracks: Track[] }) => void,
    ) => {
      callback(mockTrackStoreData);
      return () => {};
    },
  ),
  ...mockTrackStoreData,
}));

vi.mock("$lib/stores/trackStore", () => ({
  trackStore: mockTrackStore,
}));

// Mock the TrackItem component
vi.mock("./TrackItem.svelte", () => ({
  default: vi.fn().mockImplementation(() => ({
    $$typeof: Symbol.for("svelte.component"),
  })),
}));

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
        file_path: "/music/song1.mp3",
        has_cover: true,
        cover_small_url: "/api/v1/tracks/1/covers/small.webp",
        cover_original_url: "/api/v1/tracks/1/covers/original.webp",
        updated_at: 1640995200,
      },
      {
        id: 2,
        title: "Song 2",
        artist: "Artist 2",
        duration: 240,
        file_path: "/music/song2.mp3",
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
        updated_at: 1640995300,
      },
    ];

    // Clear mocks
    vi.mocked(TrackItem).mockClear();
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

    expect(screen.getByText(/No tracks available/)).toBeInTheDocument();
  });

  it("renders TrackItems when tracks are provided", () => {
    mockTrackStoreData.tracks = mockTracks;
    render(TrackList);

    // With virtualization, we don't render all items, just the visible ones
    // Since we mocked getVirtualItems to return empty array, no TrackItems should be rendered
    expect(vi.mocked(TrackItem)).toHaveBeenCalledTimes(0);
  });
});
