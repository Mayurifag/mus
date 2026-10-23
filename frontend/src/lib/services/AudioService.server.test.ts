import { expect, it, vi } from "vitest";
import { AudioService } from "./AudioService";
import type { Track } from "$lib/types";

vi.mock("svelte-sonner", () => ({ toast: { info: vi.fn() } }));

it("can update the source without a browser document", () => {
  const audio = {
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    pause: vi.fn(),
    load: vi.fn(),
  } as unknown as HTMLAudioElement;
  const service = new AudioService(audio);
  const track: Track = {
    id: "1",
    title: "Song",
    artist: "Artist",
    duration: 180,
    filename: "song.mp3",
    updated_at: 1,
    has_cover: false,
    cover_small_url: null,
    cover_original_url: null,
    hls_url: "/api/v1/tracks/1/hls/1/index.m3u8",
  };
  try {
    expect(typeof document).toBe("undefined");
    expect(() => service.updateAudioSource(track, false)).not.toThrow();
  } finally {
    service.destroy();
  }
});
