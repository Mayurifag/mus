import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { trackStore } from "./trackStore";
import type { Track } from "$lib/types";

describe("trackStore", () => {
  const mockTracks: Track[] = [
    {
      id: 1,
      title: "Test Track 1",
      artist: "Test Artist 1",
      duration: 180,
      file_path: "/path/to/file1.mp3",
      added_at: 1615478400,
      has_cover: true,
      cover_small_url: "/api/v1/tracks/1/covers/small.webp",
      cover_original_url: "/api/v1/tracks/1/covers/original.webp",
    },
    {
      id: 2,
      title: "Test Track 2",
      artist: "Test Artist 2",
      duration: 240,
      file_path: "/path/to/file2.mp3",
      added_at: 1615478500,
      has_cover: true,
      cover_small_url: "/api/v1/tracks/2/covers/small.webp",
      cover_original_url: "/api/v1/tracks/2/covers/original.webp",
    },
    {
      id: 3,
      title: "Test Track 3",
      artist: "Test Artist 3",
      duration: 200,
      file_path: "/path/to/file3.mp3",
      added_at: 1615478600,
      has_cover: false,
      cover_small_url: null,
      cover_original_url: null,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    trackStore.reset();
  });

  it("should initialize with default values", () => {
    const state = get(trackStore);
    expect(state.tracks).toEqual([]);
    expect(state.currentTrackIndex).toBeNull();
    expect(state.playHistory).toEqual([]);
    expect(state.currentTrack).toBeNull();
    expect(state.is_shuffle).toBe(false);
  });

  it("should set tracks and maintain current track index if possible", () => {
    trackStore.setTracks(mockTracks);
    expect(get(trackStore).tracks).toEqual(mockTracks);
    expect(get(trackStore).currentTrackIndex).toBe(0);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[0]);

    trackStore.setCurrentTrackIndex(1);
    expect(get(trackStore).currentTrackIndex).toBe(1);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[1]);

    const updatedTracks = [mockTracks[1], mockTracks[2], mockTracks[0]];
    trackStore.setTracks(updatedTracks);

    expect(get(trackStore).currentTrackIndex).toBe(0);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[1]);
  });

  it("should set currentTrackIndex to null when tracks array is empty", () => {
    trackStore.setTracks(mockTracks);
    expect(get(trackStore).currentTrackIndex).toBe(0);

    trackStore.setTracks([]);
    expect(get(trackStore).tracks).toEqual([]);
    expect(get(trackStore).currentTrackIndex).toBeNull();
  });

  it("should update current track index correctly", () => {
    trackStore.setTracks(mockTracks);

    trackStore.setCurrentTrackIndex(2);
    expect(get(trackStore).currentTrackIndex).toBe(2);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[2]);

    trackStore.setCurrentTrackIndex(null);
    expect(get(trackStore).currentTrackIndex).toBeNull();
    expect(get(trackStore).currentTrack).toBeNull();

    trackStore.setCurrentTrackIndex(10);
    expect(get(trackStore).currentTrackIndex).toBe(10);
  });

  it("should play track and add previous track to history", () => {
    trackStore.setTracks(mockTracks);

    trackStore.playTrack(0);
    expect(get(trackStore).currentTrackIndex).toBe(0);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[0]);
    expect(get(trackStore).playHistory).toEqual([mockTracks[0]]);

    trackStore.playTrack(1);
    expect(get(trackStore).currentTrackIndex).toBe(1);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[1]);
    expect(get(trackStore).playHistory).toEqual([mockTracks[0], mockTracks[0]]);

    trackStore.playTrack(2);
    expect(get(trackStore).currentTrackIndex).toBe(2);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[2]);
    expect(get(trackStore).playHistory).toEqual([
      mockTracks[0],
      mockTracks[0],
      mockTracks[1],
    ]);
  });

  it("should handle nextTrack in regular (non-shuffle) mode", () => {
    trackStore.setTracks(mockTracks);
    trackStore.setCurrentTrackIndex(0);

    trackStore.nextTrack();
    expect(get(trackStore).currentTrackIndex).toBe(1);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[1]);

    trackStore.setCurrentTrackIndex(2);

    trackStore.nextTrack();
    expect(get(trackStore).currentTrackIndex).toBe(0);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[0]);
  });

  it("should handle previousTrack in regular (non-shuffle) mode", () => {
    trackStore.setTracks(mockTracks);
    trackStore.setCurrentTrackIndex(1);

    trackStore.previousTrack();
    expect(get(trackStore).currentTrackIndex).toBe(0);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[0]);

    trackStore.setCurrentTrackIndex(0);

    trackStore.previousTrack();
    expect(get(trackStore).currentTrackIndex).toBe(2);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[2]);
  });

  it("should toggle shuffle state", () => {
    expect(get(trackStore).is_shuffle).toBe(false);
    trackStore.toggleShuffle();
    expect(get(trackStore).is_shuffle).toBe(true);
    trackStore.toggleShuffle();
    expect(get(trackStore).is_shuffle).toBe(false);
  });

  it("should set shuffle state", () => {
    trackStore.setShuffle(true);
    expect(get(trackStore).is_shuffle).toBe(true);
    trackStore.setShuffle(false);
    expect(get(trackStore).is_shuffle).toBe(false);
  });
});
