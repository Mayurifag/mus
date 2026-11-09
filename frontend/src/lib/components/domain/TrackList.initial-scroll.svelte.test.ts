import {
  afterAll,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import { tick } from "svelte";
import type { Track } from "$lib/types";
import { trackStore } from "$lib/stores/trackStore";
import TrackList from "./TrackList.svelte";

const track: Track = {
  id: "1",
  title: "Song",
  artist: "Artist",
  duration: 180,
  filename: "song.mp3",
  has_cover: false,
  cover_small_url: null,
  cover_original_url: null,
  hls_url: "/song.m3u8",
  updated_at: 1,
};
const scrollIntoView = vi.fn();
const originalScrollIntoView = Object.getOwnPropertyDescriptor(
  HTMLElement.prototype,
  "scrollIntoView",
);

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
    configurable: true,
    value: scrollIntoView,
  });
});

afterAll(() => {
  if (originalScrollIntoView) {
    Object.defineProperty(
      HTMLElement.prototype,
      "scrollIntoView",
      originalScrollIntoView,
    );
  } else {
    Reflect.deleteProperty(HTMLElement.prototype, "scrollIntoView");
  }
});

beforeEach(() => {
  trackStore.reset();
  scrollIntoView.mockClear();
});

describe("TrackList initial scroll", () => {
  it("scrolls to the restored track after the library loads", async () => {
    render(TrackList);
    trackStore.setTracks([track, { ...track, id: "2" }]);
    trackStore.setCurrentTrackIndex(1);

    await waitFor(() => expect(scrollIntoView).toHaveBeenCalledTimes(1));
    expect(scrollIntoView.mock.contexts[0]).toBe(
      document.getElementById("track-item-2"),
    );
  });

  it("does not scroll when the first track is selected after an empty library loads", async () => {
    render(TrackList);
    trackStore.setTracks([]);
    await tick();
    trackStore.addTrack(track);
    await tick();
    trackStore.playTrack(0);

    await waitFor(() =>
      expect(screen.getByTestId("track-progress-slider")).toBeTruthy(),
    );
    await tick();
    expect(scrollIntoView).not.toHaveBeenCalled();
  });
});
