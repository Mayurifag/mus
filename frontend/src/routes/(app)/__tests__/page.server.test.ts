import { describe, it, expect, vi } from "vitest";
import Page from "../+page.svelte";

// Mock dependencies
vi.mock("$lib/stores/trackStore", () => ({
  trackStore: {
    subscribe: vi.fn(),
  },
}));

vi.mock("$lib/components/domain/TrackList.svelte", () => ({
  default: {},
}));

describe("page.svelte", () => {
  it("should be defined", () => {
    expect(Page).toBeDefined();
  });

  it("should be a function", () => {
    expect(typeof Page).toBe("function");
  });
});
