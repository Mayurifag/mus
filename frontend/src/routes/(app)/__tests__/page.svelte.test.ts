import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";
import type { Track } from "$lib/types";
import Page from "../+page.svelte";

// Mock trackStore
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    subscribe: vi.fn().mockImplementation((callback) => {
      callback({ tracks: mockTracks });
      return () => {};
    }),
    setTracks: vi.fn(),
    setCurrentTrackIndex: vi.fn(),
    playTrack: vi.fn(),
    nextTrack: vi.fn(),
    previousTrack: vi.fn(),
    reset: vi.fn(),
  },
}));

// Mock global data
const mockTracks: Track[] = [];

describe("(app) Page component", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset mock tracks for each test
    mockTracks.length = 0;
  });

  it('should display "No tracks found" message when library is empty', () => {
    // Arrange - mockTracks is already empty

    // Act
    render(Page);

    // Assert
    expect(screen.getByText(/No tracks found/i)).toBeInTheDocument();
    expect(screen.queryByText(/Found \d+ tracks/i)).not.toBeInTheDocument();
  });

  it("should display track count when tracks are available", () => {
    // Arrange - Add mock tracks
    mockTracks.push(
      {
        id: 1,
        title: "Test Track 1",
        artist: "Artist 1",
        duration: 180,
        file_path: "/path/to/track1.mp3",
        added_at: Date.now(),
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
      },
      {
        id: 2,
        title: "Test Track 2",
        artist: "Artist 2",
        duration: 240,
        file_path: "/path/to/track2.mp3",
        added_at: Date.now(),
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
      },
    );

    // Act
    render(Page);

    // Assert
    expect(screen.getByText(/Found 2 tracks/i)).toBeInTheDocument();
    expect(screen.queryByText(/No tracks found/i)).not.toBeInTheDocument();
  });
});
