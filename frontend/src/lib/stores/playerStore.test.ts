import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import { playerStore } from "./playerStore";
import type { Track } from "$lib/types";

describe("playerStore", () => {
  const mockTrack: Track = {
    id: 1,
    title: "Test Track",
    artist: "Test Artist",
    duration: 180,
    file_path: "/path/to/file.mp3",
    added_at: 1615478400,
    has_cover: true,
    cover_small_url: "/api/v1/tracks/1/covers/small.webp",
    cover_original_url: "/api/v1/tracks/1/covers/original.webp",
  };

  beforeEach(() => {
    playerStore.reset();
  });

  it("should initialize with default values", () => {
    const state = get(playerStore);
    expect(state.currentTrack).toBeNull();
    expect(state.isPlaying).toBe(false);
    expect(state.currentTime).toBe(0);
    expect(state.duration).toBe(0);
    expect(state.volume).toBe(1.0);
    expect(state.isMuted).toBe(false);
    expect(state.is_shuffle).toBe(false);
    expect(state.is_repeat).toBe(false);
  });

  it("should toggle shuffle state", () => {
    expect(get(playerStore).is_shuffle).toBe(false);
    playerStore.toggleShuffle();
    expect(get(playerStore).is_shuffle).toBe(true);
    playerStore.toggleShuffle();
    expect(get(playerStore).is_shuffle).toBe(false);
  });

  it("should toggle repeat state", () => {
    expect(get(playerStore).is_repeat).toBe(false);
    playerStore.toggleRepeat();
    expect(get(playerStore).is_repeat).toBe(true);
    playerStore.toggleRepeat();
    expect(get(playerStore).is_repeat).toBe(false);
  });

  it("should set track and reset currentTime", () => {
    playerStore.setTrack(mockTrack);
    const state = get(playerStore);
    expect(state.currentTrack).toEqual(mockTrack);
    expect(state.duration).toBe(mockTrack.duration);
    expect(state.currentTime).toBe(0);

    // Set a non-zero currentTime
    playerStore.setCurrentTime(30);
    expect(get(playerStore).currentTime).toBe(30);

    // Setting the same track again should reset currentTime
    playerStore.setTrack(mockTrack);
    expect(get(playerStore).currentTime).toBe(0);
  });

  it("should set and control volume correctly", () => {
    // Test volume limits
    playerStore.setVolume(1.5); // Greater than max
    expect(get(playerStore).volume).toBe(1.0);

    playerStore.setVolume(-0.5); // Less than min
    expect(get(playerStore).volume).toBe(0);

    playerStore.setVolume(0.75);
    expect(get(playerStore).volume).toBe(0.75);
  });

  it("should toggle and set mute state correctly", () => {
    expect(get(playerStore).isMuted).toBe(false);

    playerStore.toggleMute();
    expect(get(playerStore).isMuted).toBe(true);

    playerStore.setMuted(false);
    expect(get(playerStore).isMuted).toBe(false);

    playerStore.setMuted(true);
    expect(get(playerStore).isMuted).toBe(true);
  });
});
