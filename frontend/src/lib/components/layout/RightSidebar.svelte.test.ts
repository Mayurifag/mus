import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
import { get } from "svelte/store";
import RightSidebar from "./RightSidebar.svelte";
import { trackStore } from "$lib/stores/trackStore";
import type { Track } from "$lib/types";

// Mock the API client
vi.mock("$lib/services/apiClient", () => ({
  fetchErroredTracks: vi.fn().mockResolvedValue([]),
  fetchSystemInfo: vi.fn().mockResolvedValue({
    app_date: "2026-05-11",
    commit_sha: null,
    yt_dlp_version: null,
  }),
  updateYtDlp: vi.fn().mockResolvedValue({ yt_dlp_version: null, output: "" }),
}));

describe("RightSidebar", () => {
  beforeEach(() => {
    trackStore.reset();
  });

  it("should be defined", () => {
    expect(RightSidebar).toBeDefined();
  });

  it("should render as a div element", () => {
    const { container } = render(RightSidebar);

    expect(container.querySelector("div")).toBeInTheDocument();
  });

  it("should have correct styling classes", () => {
    const { container } = render(RightSidebar);

    const div = container.querySelector("div");
    expect(div).toHaveClass("h-full", "w-full");
  });

  it("should render Lucide Play icon in timeline when shuffle is enabled", () => {
    // Setup mock track data
    const mockTrack1: Track = {
      id: 1,
      title: "Test Track 1",
      artist: "Test Artist",
      duration: 180,
      file_path: "/test/path1.mp3",
      updated_at: Date.now(),
      has_cover: false,
      cover_small_url: null,
      cover_original_url: null,
    };

    const mockTrack2: Track = {
      id: 2,
      title: "Test Track 2",
      artist: "Test Artist",
      duration: 200,
      file_path: "/test/path2.mp3",
      updated_at: Date.now(),
      has_cover: false,
      cover_small_url: null,
      cover_original_url: null,
    };

    // Enable shuffle and set up play history with enough tracks
    trackStore.setTracks([mockTrack1, mockTrack2]);
    trackStore.toggleShuffle();
    trackStore.playTrack(0);
    trackStore.nextTrack(); // This should add to history

    const { container } = render(RightSidebar);

    // Check that the Play icon is rendered (it should have the lucide-play class)
    const playIcon = container.querySelector('svg[class*="lucide-play"]');
    expect(playIcon).toBeInTheDocument();
    expect(playIcon).toHaveClass("h-3", "w-3", "text-accent");
  });

  it("should render top artists and filter by artist", async () => {
    const tracks: Track[] = [
      {
        id: 1,
        title: "Song 1",
        artist: "Artist A; Artist B",
        duration: 180,
        file_path: "/test/path1.mp3",
        updated_at: Date.now(),
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
      },
      {
        id: 2,
        title: "Song 2",
        artist: "Artist A",
        duration: 200,
        file_path: "/test/path2.mp3",
        updated_at: Date.now(),
        has_cover: false,
        cover_small_url: null,
        cover_original_url: null,
      },
    ];

    trackStore.setTracks(tracks);
    render(RightSidebar);

    expect(screen.getByText("Artists")).toBeInTheDocument();
    expect(screen.getByText("Artist A")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();

    await fireEvent.click(screen.getByText("Artist A"));

    expect(get(trackStore).selectedArtist).toBe("Artist A");
  });
});
