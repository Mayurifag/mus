import { afterEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { trackStore } from "$lib/stores/trackStore";
import type { AudioService } from "$lib/services/AudioService";
import type { Track } from "$lib/types";
import { restorePlayerState, savePlayerState } from "./playerStateService";

const mocks = vi.hoisted(() => ({
  sendPlayerStateBeacon: vi.fn(),
}));

vi.mock("$lib/services/apiClient", () => ({
  sendPlayerStateBeacon: mocks.sendPlayerStateBeacon,
}));

function track(id: string): Track {
  return {
    id,
    title: `Track ${id}`,
    artist: "Artist",
    duration: 120,
    filename: `track-${id}.mp3`,
    updated_at: Number(id),
    has_cover: false,
    cover_small_url: null,
    cover_original_url: null,
    hls_url: `/api/v1/tracks/${id}/hls/${id}/index.m3u8`,
  };
}

describe("playerStateService", () => {
  afterEach(() => {
    trackStore.reset();
    vi.resetAllMocks();
  });

  it("restores the global player state to the saved track", () => {
    const tracks = [track("1"), track("2")];
    trackStore.setTracks(tracks);
    const audioService = {
      initializeState: vi.fn(),
      updateAudioSource: vi.fn(),
    } as unknown as AudioService;

    const restoredId = restorePlayerState(audioService, tracks, {
      current_track_id: "2",
      progress_seconds: 45.5,
      volume_level: 0.7,
      is_muted: true,
      is_shuffle: true,
      is_repeat: true,
    });

    expect(restoredId).toBe("2");
    expect(audioService.initializeState).toHaveBeenCalledWith(0.7, true, true);
    expect(audioService.updateAudioSource).toHaveBeenCalledWith(
      tracks[1],
      false,
      45.5,
    );
    expect(get(trackStore).currentTrack?.id).toBe("2");
    expect(get(trackStore).is_shuffle).toBe(true);
  });

  it("saves one global player state payload for the current track", () => {
    const currentTrack = track("2");
    const audioService = {
      currentTime: 12.25,
      volume: 0.4,
      isMuted: false,
      isRepeat: true,
    } as unknown as AudioService;

    savePlayerState(audioService, currentTrack, true);

    expect(mocks.sendPlayerStateBeacon).toHaveBeenCalledWith({
      current_track_id: "2",
      progress_seconds: 12.25,
      volume_level: 0.4,
      is_muted: false,
      is_shuffle: true,
      is_repeat: true,
    });
  });
});
