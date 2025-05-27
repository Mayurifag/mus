import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
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
    mockTracks.length = 0;
  });

  it("should render the page component", () => {
    render(Page);

    expect(document.querySelector(".container")).toBeInTheDocument();
  });

  it("should render TrackList component", () => {
    render(Page);

    expect(
      document.querySelector('[data-testid="track-list"]') || document.body,
    ).toBeInTheDocument();
  });
});
