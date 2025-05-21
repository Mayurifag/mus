/**
 * @vitest-environment jsdom
 * @vitest-environment-options { "url": "http://localhost:5173" }
 */
import { describe, it, expect, vi } from "vitest";
import Page from "../+page.svelte";

// Mock dependencies
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    subscribe: vi.fn((callback) => {
      callback({ tracks: [], currentTrackIndex: -1 });
      return () => {};
    }),
  },
}));

vi.mock("$lib/components/domain/TrackList.svelte", () => ({
  default: {
    render: () => document.createElement("div"),
  },
}));

describe("page.svelte", () => {
  it("initializes correctly", () => {
    // Just verify that Page exists
    expect(Page).toBeDefined();
    expect(typeof Page).toBe("function");
  });
});
