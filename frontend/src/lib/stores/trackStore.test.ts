import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { get } from "svelte/store";
import { artistCountsStore, trackStore } from "./trackStore";
import type { Track } from "$lib/types";

describe("trackStore", () => {
  const mockTracks: Track[] = [
    {
      id: 1,
      title: "Test Track 1",
      artist: "Test Artist 1",
      duration: 180,
      filename: "track1.mp3",
      has_cover: true,
      cover_small_url: "/api/v1/tracks/1/covers/small.webp",
      cover_original_url: "/api/v1/tracks/1/covers/original.webp",
      updated_at: 1640995200,
    },
    {
      id: 2,
      title: "Test Track 2",
      artist: "Test Artist 2",
      duration: 240,
      filename: "track2.mp3",
      has_cover: true,
      cover_small_url: "/api/v1/tracks/2/covers/small.webp",
      cover_original_url: "/api/v1/tracks/2/covers/original.webp",
      updated_at: 1640995300,
    },
    {
      id: 3,
      title: "Test Track 3",
      artist: "Test Artist 3",
      duration: 200,
      filename: "track3.mp3",
      has_cover: false,
      cover_small_url: null,
      cover_original_url: null,
      updated_at: 1640995400,
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
    expect(state.historyPosition).toBe(-1);
    expect(state.currentTrack).toBeNull();
    expect(state.is_shuffle).toBe(false);
    expect(state.selectedArtist).toBeNull();
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

  it("should derive artist counts from tracks", () => {
    const tracks: Track[] = [
      { ...mockTracks[0], artist: "Artist A; Artist B; Artist A" },
      { ...mockTracks[1], artist: "Artist A" },
      { ...mockTracks[2], artist: "Artist C; __proto__; toString" },
    ];

    trackStore.setTracks(tracks);

    let counts = get(artistCountsStore);
    expect(counts).toMatchObject({
      "Artist A": 2,
      "Artist B": 1,
      "Artist C": 1,
      toString: 1,
    });
    expect(Object.getOwnPropertyDescriptor(counts, "__proto__")?.value).toBe(1);

    trackStore.updateTrack({ ...tracks[0], artist: "Artist B" });

    counts = get(artistCountsStore);
    expect(counts).toMatchObject({
      "Artist A": 1,
      "Artist B": 1,
      "Artist C": 1,
      toString: 1,
    });
    expect(Object.getOwnPropertyDescriptor(counts, "__proto__")?.value).toBe(1);
  });

  it("should set currentTrackIndex to null when tracks array is empty", () => {
    trackStore.setTracks(mockTracks);
    expect(get(trackStore).currentTrackIndex).toBe(0);

    trackStore.setTracks([]);
    expect(get(trackStore).tracks).toEqual([]);
    expect(get(trackStore).currentTrackIndex).toBeNull();
  });

  it("should not auto-select first track when current track already exists", () => {
    // First set tracks normally (auto-selects first track)
    trackStore.setTracks(mockTracks);
    expect(get(trackStore).currentTrackIndex).toBe(0);

    // Set current track to something else
    trackStore.setCurrentTrackIndex(1);
    expect(get(trackStore).currentTrackIndex).toBe(1);

    // Setting tracks again should preserve the current track if it exists in new tracks
    trackStore.setTracks(mockTracks);
    expect(get(trackStore).currentTrackIndex).toBe(1);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[1]);
  });

  it("should replace auto-selected track when setting different track during initialization", () => {
    // Simulate page initialization: auto-select first track
    trackStore.setTracks(mockTracks);
    let state = get(trackStore);
    expect(state.currentTrackIndex).toBe(0);
    expect(state.playHistory).toEqual([mockTracks[0]]);
    expect(state.historyPosition).toBe(0);

    // Simulate backend restoration setting a different track
    trackStore.setCurrentTrackIndex(2);
    state = get(trackStore);
    expect(state.currentTrackIndex).toBe(2);
    expect(state.currentTrack).toEqual(mockTracks[2]);
    // History should contain only the backend track, not both
    expect(state.playHistory).toEqual([mockTracks[2]]);
    expect(state.historyPosition).toBe(0);
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
    expect(get(trackStore).currentTrackIndex).toBeNull();
  });

  it("should play track and reset history", () => {
    trackStore.setTracks(mockTracks);

    // Playing the same track (0) that's already current should not change anything
    trackStore.playTrack(0);
    expect(get(trackStore).currentTrackIndex).toBe(0);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[0]);
    expect(get(trackStore).playHistory).toEqual([mockTracks[0]]);
    expect(get(trackStore).historyPosition).toBe(0);

    // Playing a different track should reset history to only contain the new track
    trackStore.playTrack(1);
    expect(get(trackStore).currentTrackIndex).toBe(1);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[1]);
    expect(get(trackStore).playHistory).toEqual([mockTracks[1]]);
    expect(get(trackStore).historyPosition).toBe(0);
    expect(get(trackStore).playRequestId).toBe(1);

    trackStore.playTrack(2);
    expect(get(trackStore).currentTrackIndex).toBe(2);
    expect(get(trackStore).currentTrack).toEqual(mockTracks[2]);
    expect(get(trackStore).playHistory).toEqual([mockTracks[2]]);
    expect(get(trackStore).historyPosition).toBe(0);
    expect(get(trackStore).playRequestId).toBe(2);
  });

  it("should add tracks from scan events without duplicating existing tracks", () => {
    trackStore.setTracks(mockTracks);

    const scannedTrack: Track = {
      id: 4,
      title: "Scanned Track",
      artist: "Scanned Artist",
      duration: 220,
      filename: "scanned.mp3",
      has_cover: false,
      cover_small_url: null,
      cover_original_url: null,
      updated_at: 1640995500,
    };

    trackStore.addTrack(scannedTrack);
    expect(get(trackStore).tracks).toEqual([scannedTrack, ...mockTracks]);
    expect(get(trackStore).currentTrackIndex).toBe(1);

    const updatedScannedTrack = {
      ...scannedTrack,
      title: "Updated Scanned Track",
    };
    trackStore.addTrack(updatedScannedTrack);

    const state = get(trackStore);
    expect(state.tracks).toHaveLength(4);
    expect(state.tracks[0]).toEqual(updatedScannedTrack);
    expect(state.currentTrackIndex).toBe(1);
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

  it("should mark nextTrack as a play request when requested", () => {
    trackStore.setTracks(mockTracks);
    trackStore.setCurrentTrackIndex(0);

    trackStore.nextTrack(true);

    expect(get(trackStore).currentTrackIndex).toBe(1);
    expect(get(trackStore).playRequestId).toBe(1);
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

  it("should set and clear artist filter", () => {
    trackStore.setArtistFilter("Test Artist 1");
    expect(get(trackStore).selectedArtist).toBe("Test Artist 1");

    trackStore.clearArtistFilter();
    expect(get(trackStore).selectedArtist).toBeNull();
  });

  it("should keep next and previous navigation inside selected artist", () => {
    const artistTracks: Track[] = [
      { ...mockTracks[0], artist: "Artist A" },
      { ...mockTracks[1], artist: "Artist B" },
      { ...mockTracks[2], artist: "Artist A; Artist C" },
    ];

    trackStore.setTracks(artistTracks);
    trackStore.setCurrentTrackIndex(0);
    trackStore.setArtistFilter("Artist A");

    trackStore.nextTrack();
    expect(get(trackStore).currentTrackIndex).toBe(2);
    expect(get(trackStore).currentTrack).toEqual(artistTracks[2]);

    trackStore.nextTrack();
    expect(get(trackStore).currentTrackIndex).toBe(0);
    expect(get(trackStore).currentTrack).toEqual(artistTracks[0]);

    trackStore.previousTrack();
    expect(get(trackStore).currentTrackIndex).toBe(2);
    expect(get(trackStore).currentTrack).toEqual(artistTracks[2]);
  });

  it("should keep shuffle navigation inside selected artist", () => {
    const artistTracks: Track[] = [
      { ...mockTracks[0], artist: "Artist A" },
      { ...mockTracks[1], artist: "Artist B" },
      { ...mockTracks[2], artist: "Artist A" },
    ];
    vi.spyOn(Math, "random").mockReturnValue(0);

    trackStore.setTracks(artistTracks);
    trackStore.setCurrentTrackIndex(0);
    trackStore.setArtistFilter("Artist A");
    trackStore.setShuffle(true);

    trackStore.nextTrack();
    expect(get(trackStore).currentTrackIndex).toBe(2);
    expect(get(trackStore).currentTrack).toEqual(artistTracks[2]);

    vi.restoreAllMocks();
  });

  describe("shuffle mode navigation", () => {
    beforeEach(() => {
      trackStore.reset();
      trackStore.setTracks(mockTracks);
      trackStore.setCurrentTrackIndex(0);
      trackStore.setShuffle(true);
      vi.spyOn(Math, "random").mockReturnValue(0.5);
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it("should maintain consistent forward/backward navigation in shuffle mode", () => {
      // Start at track 0
      expect(get(trackStore).currentTrackIndex).toBe(0);
      expect(get(trackStore).playHistory).toEqual([mockTracks[0]]);
      expect(get(trackStore).historyPosition).toBe(0);

      // Go to next track (should be random)
      trackStore.nextTrack();
      const firstNextState = get(trackStore);
      const firstNextTrack = firstNextState.currentTrack;
      expect(firstNextState.playHistory.length).toBeGreaterThan(0);

      // Go back to previous track
      trackStore.previousTrack();
      const backState = get(trackStore);
      expect(backState.currentTrackIndex).toBe(0);
      expect(backState.currentTrack).toEqual(mockTracks[0]);

      // Go forward again - should return to the same track as before
      trackStore.nextTrack();
      const forwardAgainState = get(trackStore);
      expect(forwardAgainState.currentTrack).toEqual(firstNextTrack);
    });

    it("should update history position when manually selecting a track", () => {
      // Navigate to create some history
      trackStore.nextTrack();
      trackStore.previousTrack();

      const stateWithHistory = get(trackStore);
      expect(stateWithHistory.historyPosition).toBeGreaterThanOrEqual(0);

      // Manually play a track
      trackStore.playTrack(2);

      const stateAfterManualPlay = get(trackStore);
      expect(stateAfterManualPlay.currentTrackIndex).toBe(2);
      // History position should point to the newly added track
      expect(stateAfterManualPlay.historyPosition).toBe(
        stateAfterManualPlay.playHistory.length - 1,
      );
    });

    it("should generate new random track when at end of history", () => {
      // Go to next track twice to create some history
      trackStore.nextTrack();
      trackStore.nextTrack();

      const state = get(trackStore);
      expect(state.historyPosition).toBe(state.playHistory.length - 1);

      // Next track should be random since at end of history
      const currentTrackBefore = state.currentTrack;
      trackStore.nextTrack();

      const stateAfter = get(trackStore);
      expect(stateAfter.currentTrack).not.toEqual(currentTrackBefore);
      expect(stateAfter.playHistory.length).toBeGreaterThan(
        state.playHistory.length,
      );
    });

    it("should handle multiple back/forward operations correctly", () => {
      // Create a sequence: Track 0 -> Track A -> Track B
      trackStore.nextTrack(); // 0 -> A
      const trackA = get(trackStore).currentTrack;

      trackStore.nextTrack(); // A -> B
      const trackB = get(trackStore).currentTrack;

      // Go back twice: B -> A -> 0
      trackStore.previousTrack(); // B -> A
      expect(get(trackStore).currentTrack).toEqual(trackA);

      trackStore.previousTrack(); // A -> 0
      expect(get(trackStore).currentTrackIndex).toBe(0);

      // Go forward twice: 0 -> A -> B
      trackStore.nextTrack(); // 0 -> A
      expect(get(trackStore).currentTrack).toEqual(trackA);

      trackStore.nextTrack(); // A -> B
      expect(get(trackStore).currentTrack).toEqual(trackB);
    });

    it("should reset history position when tracks are updated", () => {
      // Create some history
      trackStore.nextTrack();
      trackStore.previousTrack();

      expect(get(trackStore).historyPosition).toBe(0);

      // Update tracks
      trackStore.setTracks([mockTracks[0], mockTracks[1]]);

      expect(get(trackStore).historyPosition).toBe(0);
    });

    it("should handle clicking back too much and maintain next history", () => {
      // Build up shuffle history: Track 0 -> A -> B -> C
      trackStore.nextTrack(); // 0 -> A
      const trackA = get(trackStore).currentTrack;

      trackStore.nextTrack(); // A -> B
      const trackB = get(trackStore).currentTrack;

      trackStore.nextTrack(); // B -> C
      const trackC = get(trackStore).currentTrack;

      // At this point: history = [Track0, TrackA, TrackB, TrackC], position = 3
      let state = get(trackStore);
      expect(state.playHistory.length).toBe(4);
      expect(state.historyPosition).toBe(3);
      expect(state.currentTrack).toEqual(trackC);

      // Go back multiple times
      trackStore.previousTrack(); // C -> B
      state = get(trackStore);
      expect(state.historyPosition).toBe(2);
      expect(state.currentTrack).toEqual(trackB);

      trackStore.previousTrack(); // B -> A
      state = get(trackStore);
      expect(state.historyPosition).toBe(1);
      expect(state.currentTrack).toEqual(trackA);

      trackStore.previousTrack(); // A -> Track0
      state = get(trackStore);
      expect(state.historyPosition).toBe(0);
      expect(state.currentTrack).toEqual(mockTracks[0]);

      // Try to go back again - should continue sequential navigation (Track0 -> Track2)
      trackStore.previousTrack(); // Track0 -> Track2 (wrap-around)
      state = get(trackStore);
      expect(state.historyPosition).toBe(0);
      expect(state.currentTrack).toEqual(mockTracks[2]); // Track 2 (wrapped around)

      // The history should now be extended with the new track
      expect(state.playHistory.length).toBe(5);
      expect(state.playHistory[0]).toEqual(mockTracks[2]); // New track at position 0
      expect(state.playHistory[1]).toEqual(mockTracks[0]); // Previous tracks shifted
      expect(state.playHistory[2]).toEqual(trackA);
      expect(state.playHistory[3]).toEqual(trackB);
      expect(state.playHistory[4]).toEqual(trackC);

      // Now going forward should work through the extended history
      trackStore.nextTrack(); // Track2 -> Track0
      state = get(trackStore);
      expect(state.historyPosition).toBe(1);
      expect(state.currentTrack).toEqual(mockTracks[0]);

      trackStore.nextTrack(); // Track0 -> A
      state = get(trackStore);
      expect(state.historyPosition).toBe(2);
      expect(state.currentTrack).toEqual(trackA);
    });

    it("should fall back to sequential previous when shuffle is on but no history exists", () => {
      // Reset to have no history
      trackStore.reset();
      trackStore.setTracks(mockTracks); // This will auto-select track 0
      trackStore.setCurrentTrackIndex(1); // Change to track 1
      trackStore.setShuffle(true);

      // At this point: currentTrackIndex = 1, playHistory should contain only track 1 (auto-selected track 0 was replaced)
      let state = get(trackStore);
      expect(state.currentTrackIndex).toBe(1);
      expect(state.playHistory).toEqual([mockTracks[1]]); // Only the manually selected track 1
      expect(state.historyPosition).toBe(0); // Pointing to track 1 at position 0

      // Press previous - should go to track 0 (sequential previous)
      trackStore.previousTrack();
      state = get(trackStore);
      expect(state.currentTrackIndex).toBe(0);
      expect(state.currentTrack).toEqual(mockTracks[0]);
      // History should be minimal 2-track: [previousTrack, originalTrack]
      expect(state.playHistory).toEqual([mockTracks[0], mockTracks[1]]);
      expect(state.historyPosition).toBe(0); // Point to the previous track
    });

    it("should fall back to sequential previous when shuffle is on and at track 0", () => {
      // Reset to have no history
      trackStore.reset();
      trackStore.setTracks(mockTracks); // This will auto-select track 0
      trackStore.setShuffle(true);

      // Press previous - should wrap to last track (track 2)
      trackStore.previousTrack();
      const state = get(trackStore);
      expect(state.currentTrackIndex).toBe(2);
      expect(state.currentTrack).toEqual(mockTracks[2]);
      // History should be minimal 2-track: [previousTrack, originalTrack]
      expect(state.playHistory).toEqual([mockTracks[2], mockTracks[0]]);
      expect(state.historyPosition).toBe(0); // Point to the previous track
    });

    it("should allow returning to original track after fallback previous", () => {
      // Reset to have no history
      trackStore.reset();
      trackStore.setTracks(mockTracks); // This will auto-select track 0
      trackStore.setCurrentTrackIndex(1); // Change to track 1
      trackStore.setShuffle(true);

      // Press previous - creates minimal history [track0, track1], position 0
      trackStore.previousTrack();
      let state = get(trackStore);
      expect(state.currentTrackIndex).toBe(0);
      expect(state.playHistory).toEqual([mockTracks[0], mockTracks[1]]);
      expect(state.historyPosition).toBe(0);

      // Press next - should return to original track 1
      trackStore.nextTrack();
      state = get(trackStore);
      expect(state.currentTrackIndex).toBe(1);
      expect(state.currentTrack).toEqual(mockTracks[1]);
      expect(state.playHistory).toEqual([mockTracks[0], mockTracks[1]]);
      expect(state.historyPosition).toBe(1); // Now pointing to track 1
    });

    it("should allow continued backward navigation after initial fallback", () => {
      // Start at track 1 - simpler test case
      trackStore.reset();
      trackStore.setTracks(mockTracks); // Auto-selects track 0
      trackStore.setCurrentTrackIndex(1); // Set to track 1
      trackStore.setShuffle(true);

      // First previous: Track 1 → Track 0 (creates minimal history)
      trackStore.previousTrack();
      let state = get(trackStore);
      expect(state.currentTrackIndex).toBe(0);
      expect(state.playHistory).toEqual([mockTracks[0], mockTracks[1]]);
      expect(state.historyPosition).toBe(0);

      // Second previous: Track 0 → Track 2 (wrap-around, extends history)
      trackStore.previousTrack();
      state = get(trackStore);
      expect(state.currentTrackIndex).toBe(2);
      expect(state.playHistory).toEqual([
        mockTracks[2],
        mockTracks[0],
        mockTracks[1],
      ]);
      expect(state.historyPosition).toBe(0);

      // Third previous: Track 2 → Track 1 (extends history)
      trackStore.previousTrack();
      state = get(trackStore);
      expect(state.currentTrackIndex).toBe(1);
      expect(state.playHistory).toEqual([
        mockTracks[1],
        mockTracks[2],
        mockTracks[0],
        mockTracks[1],
      ]);
      expect(state.historyPosition).toBe(0);
    });

    it("should navigate forward through accumulated history after multiple backward steps", () => {
      // Start at track 1 and go backward multiple times
      trackStore.reset();
      trackStore.setTracks(mockTracks); // Auto-selects track 0
      trackStore.setCurrentTrackIndex(1); // Set to track 1
      trackStore.setShuffle(true);

      // Go backward: Track 1 → Track 0 → Track 2
      trackStore.previousTrack(); // 1 → 0
      trackStore.previousTrack(); // 0 → 2

      let state = get(trackStore);
      expect(state.currentTrackIndex).toBe(2);
      expect(state.playHistory).toEqual([
        mockTracks[2],
        mockTracks[0],
        mockTracks[1],
      ]);
      expect(state.historyPosition).toBe(0);

      // Go forward: Track 2 → Track 0
      trackStore.nextTrack();
      state = get(trackStore);
      expect(state.currentTrackIndex).toBe(0);
      expect(state.historyPosition).toBe(1);

      // Go forward: Track 0 → Track 1
      trackStore.nextTrack();
      state = get(trackStore);
      expect(state.currentTrackIndex).toBe(1);
      expect(state.historyPosition).toBe(2);
    });
  });

  describe("Shuffle mode history clearing", () => {
    beforeEach(() => {
      trackStore.reset();
      trackStore.setTracks(mockTracks);
    });

    it("should clear history when disabling shuffle via toggleShuffle", () => {
      // Enable shuffle and build some history
      trackStore.setShuffle(true);
      trackStore.nextTrack(); // This will add tracks to history
      trackStore.nextTrack();

      let state = get(trackStore);
      expect(state.is_shuffle).toBe(true);
      expect(state.playHistory.length).toBeGreaterThan(1);

      // Disable shuffle - should clear history
      trackStore.toggleShuffle();
      state = get(trackStore);
      expect(state.is_shuffle).toBe(false);
      expect(state.playHistory).toEqual([state.currentTrack]);
      expect(state.historyPosition).toBe(0);
    });

    it("should clear history when disabling shuffle via setShuffle", () => {
      // Enable shuffle and build some history
      trackStore.setShuffle(true);
      trackStore.nextTrack();
      trackStore.nextTrack();

      let state = get(trackStore);
      expect(state.is_shuffle).toBe(true);
      expect(state.playHistory.length).toBeGreaterThan(1);

      // Disable shuffle - should clear history
      trackStore.setShuffle(false);
      state = get(trackStore);
      expect(state.is_shuffle).toBe(false);
      expect(state.playHistory).toEqual([state.currentTrack]);
      expect(state.historyPosition).toBe(0);
    });

    it("should not clear history when enabling shuffle", () => {
      // Start with shuffle disabled and some manual history
      trackStore.playTrack(1);
      trackStore.playTrack(2);

      let state = get(trackStore);
      expect(state.is_shuffle).toBe(false);
      const originalHistory = [...state.playHistory];

      // Enable shuffle - should preserve history
      trackStore.setShuffle(true);
      state = get(trackStore);
      expect(state.is_shuffle).toBe(true);
      expect(state.playHistory).toEqual(originalHistory);
    });

    it("should not clear history when toggling shuffle from false to true", () => {
      // Start with shuffle disabled and some manual history
      trackStore.playTrack(1);
      trackStore.playTrack(2);

      let state = get(trackStore);
      expect(state.is_shuffle).toBe(false);
      const originalHistory = [...state.playHistory];

      // Toggle shuffle on - should preserve history
      trackStore.toggleShuffle();
      state = get(trackStore);
      expect(state.is_shuffle).toBe(true);
      expect(state.playHistory).toEqual(originalHistory);
    });
  });

  describe("Manual track selection behavior", () => {
    beforeEach(() => {
      trackStore.reset();
      trackStore.setTracks(mockTracks);
    });

    it("should reset history when manually selecting a different track", () => {
      // Initial state: Track 0 is auto-selected
      let state = get(trackStore);
      expect(state.currentTrackIndex).toBe(0);
      expect(state.playHistory).toEqual([mockTracks[0]]);
      expect(state.historyPosition).toBe(0);

      // User manually selects Track 1 - should reset history
      trackStore.playTrack(1);
      state = get(trackStore);

      expect(state.currentTrackIndex).toBe(1);
      expect(state.currentTrack).toEqual(mockTracks[1]);
      expect(state.playHistory).toEqual([mockTracks[1]]); // History reset to only new track
      expect(state.historyPosition).toBe(0);
    });

    it("should handle multiple manual track selections correctly", () => {
      // Start with Track 0
      let state = get(trackStore);
      expect(state.playHistory).toEqual([mockTracks[0]]);

      // Select Track 1 - resets history
      trackStore.playTrack(1);
      state = get(trackStore);
      expect(state.playHistory).toEqual([mockTracks[1]]);

      // Select Track 2 - resets history again
      trackStore.playTrack(2);
      state = get(trackStore);
      expect(state.playHistory).toEqual([mockTracks[2]]);
      expect(state.historyPosition).toBe(0);
    });
  });
});
