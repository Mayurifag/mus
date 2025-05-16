import { vi } from "vitest";

// Mock the trackStore
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    subscribe: vi.fn().mockImplementation((callback) => {
      callback({ currentTrackIndex: 0 });
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
        file_path: "/path/to/song1.mp3",
        added_at: Math.floor(Date.now() / 1000),
        has_cover: true,
        cover_small_url: "/api/v1/tracks/1/covers/small.webp",
        cover_original_url: "/api/v1/tracks/1/covers/original.webp",
      },
      {
        id: 2,
        title: "Song 2",
        artist: "Artist 2",
        duration: 240,
        file_path: "/path/to/song2.mp3",
        added_at: Math.floor(Date.now() / 1000),
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
      },
    ];

    // Clear mock
    vi.mocked(TrackItem).mockClear();
  });

  it("renders the track list element when tracks are provided", () => {
    render(TrackList, { tracks: mockTracks });

    const trackListElement = screen.getByTestId("track-list");
    expect(trackListElement).toBeInTheDocument();
  });

  it("renders empty state messages when no tracks are available", () => {
    render(TrackList, { tracks: [] });

    expect(screen.getByText("No tracks available")).toBeInTheDocument();
    expect(
      screen.getByText("Try scanning your music library to add tracks"),
    ).toBeInTheDocument();
  });

  it("renders the right number of TrackItems when tracks are provided", () => {
    render(TrackList, { tracks: mockTracks });

    // Check the overflow container exists
    const trackList = screen.getByTestId("track-list");
    const overflowContainer = trackList.querySelector(".overflow-y-auto");
    expect(overflowContainer).not.toBeNull();

    // Verify the mock component was called the expected number of times
    expect(vi.mocked(TrackItem)).toHaveBeenCalledTimes(mockTracks.length);
  });
});
