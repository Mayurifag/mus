import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { trackStore } from "$lib/stores/trackStore";
import {
  clearArtistFilter,
  selectArtistFilter,
} from "$lib/utils/artistFilterNavigation";

describe("artistFilterNavigation", () => {
  beforeEach(() => {
    trackStore.reset();
    vi.mocked(window.scrollTo).mockClear();
    Object.defineProperty(window, "scrollX", {
      value: 12,
      configurable: true,
    });
    Object.defineProperty(window, "scrollY", {
      value: 640,
      configurable: true,
    });
  });

  afterEach(async () => {
    await clearArtistFilter();
    trackStore.reset();
  });

  it("restores the all-songs scroll position after clearing an artist", async () => {
    selectArtistFilter("Artist A");

    Object.defineProperty(window, "scrollY", {
      value: 80,
      configurable: true,
    });

    await clearArtistFilter();

    expect(get(trackStore).selectedArtist).toBeNull();
    expect(window.scrollTo).toHaveBeenCalledWith({
      top: 640,
      left: 12,
      behavior: "auto",
    });
  });

  it("keeps the original all-songs position when switching artists", async () => {
    selectArtistFilter("Artist A");

    Object.defineProperty(window, "scrollY", {
      value: 80,
      configurable: true,
    });
    selectArtistFilter("Artist B");

    await clearArtistFilter();

    expect(window.scrollTo).toHaveBeenCalledWith({
      top: 640,
      left: 12,
      behavior: "auto",
    });
  });

  it("does not scroll when there is no saved artist return position", async () => {
    await clearArtistFilter();

    expect(window.scrollTo).not.toHaveBeenCalled();
  });
});
