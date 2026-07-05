import { describe, expect, it } from "vitest";
import type { Track } from "$lib/types";
import { displayTrackTitle, trackCategoryNames } from "./categoryMarkers";

function track(title: string, tags: string[]): Track {
  return {
    id: "1",
    title,
    artist: "Artist",
    duration: 180,
    filename: "song.mp3",
    updated_at: 1,
    has_cover: false,
    cover_small_url: null,
    cover_original_url: null,
    hls_url: "/api/v1/tracks/1/hls/1/index.m3u8",
    tags: tags.map((name) => ({ name, display_name: name })),
  };
}

describe("categoryMarkers", () => {
  it("uses backend tags as category names", () => {
    expect(trackCategoryNames(track("Song", ["cover", "unknown"]))).toEqual([
      "cover",
    ]);
  });

  it("strips active bracketed markers from display titles", () => {
    expect(
      displayTrackTitle(track("Song (Right version)", ["right-version"])),
    ).toBe("Song");
    expect(displayTrackTitle(track("Song [AI cover]", ["ai-cover"]))).toBe(
      "Song",
    );
    expect(
      displayTrackTitle(track("Song (OST mash-up)", ["ost", "mashup"])),
    ).toBe("Song");
  });

  it("strips active trailing markers from display titles", () => {
    expect(displayTrackTitle(track("Song - cover", ["cover"]))).toBe("Song");
    expect(displayTrackTitle(track("Song mashup", ["mashup"]))).toBe("Song");
    expect(
      displayTrackTitle(track("Song OST mash-up", ["ost", "mashup"])),
    ).toBe("Song");
  });

  it("keeps marker text when the backend did not tag it", () => {
    expect(displayTrackTitle(track("Song (OST)", []))).toBe("Song (OST)");
  });
});
