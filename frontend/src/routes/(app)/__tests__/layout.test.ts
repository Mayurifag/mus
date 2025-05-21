/**
 * @vitest-environment jsdom
 * @vitest-environment-options { "url": "http://localhost:5173" }
 */
import { describe, it, expect, vi } from "vitest";
import Layout from "../+layout.svelte";

// Mock dependencies
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    setTracks: vi.fn(),
    setCurrentTrackIndex: vi.fn(),
    nextTrack: vi.fn(),
    subscribe: vi.fn(),
  },
}));

vi.mock("$lib/stores/playerStore", () => ({
  playerStore: {
    setVolume: vi.fn(),
    toggleMute: vi.fn(),
    setCurrentTime: vi.fn(),
    update: vi.fn(),
    subscribe: vi.fn(),
  },
}));

vi.mock("$lib/services/eventHandlerService", () => ({
  initEventHandlerService: vi.fn(),
}));

vi.mock("$lib/services/apiClient", () => ({
  savePlayerState: vi.fn(),
  getStreamUrl: vi.fn(),
}));

vi.mock("$lib/components/layout/PlayerFooter.svelte", () => ({
  default: {},
}));

describe("layout.svelte", () => {
  it("should be defined", () => {
    expect(Layout).toBeDefined();
  });

  it("should be a function", () => {
    expect(typeof Layout).toBe("function");
  });
});
