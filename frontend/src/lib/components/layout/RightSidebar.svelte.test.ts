import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/svelte";
import RightSidebar from "./RightSidebar.svelte";
import { trackStore } from "$lib/stores/trackStore";
import type { Track } from "$lib/types";

// Mock the API client
vi.mock("$lib/services/apiClient", () => ({
  fetchErroredTracks: vi.fn().mockResolvedValue([]),
}));

describe("RightSidebar", () => {
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
});
