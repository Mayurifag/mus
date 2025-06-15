import { writable } from "svelte/store";
import type { Track } from "$lib/types";

export interface TrackStoreState {
  tracks: Track[];
  currentTrackIndex: number | null;
  playHistory: Track[];
  historyPosition: number;
  currentTrack: Track | null;
  is_shuffle: boolean;
}

const initialState: TrackStoreState = {
  tracks: [],
  currentTrackIndex: null,
  playHistory: [],
  historyPosition: -1,
  currentTrack: null,
  is_shuffle: false,
};

function createTrackStore() {
  const { subscribe, set, update } = writable<TrackStoreState>(initialState);

  // Helper functions for cleaner state management
  const createEmptyState = (tracks: Track[]): Partial<TrackStoreState> => ({
    tracks,
    currentTrackIndex: null,
    currentTrack: null,
    playHistory: [],
    historyPosition: -1,
  });

  const createInitialTrackState = (
    tracks: Track[],
    index: number,
  ): Partial<TrackStoreState> => ({
    tracks,
    currentTrackIndex: index,
    currentTrack: tracks[index],
    playHistory: [tracks[index]],
    historyPosition: 0,
  });

  const preserveCurrentTrack = (
    state: TrackStoreState,
    tracks: Track[],
    newIndex: number,
  ): Partial<TrackStoreState> => {
    const newHistory =
      state.playHistory.length > 0 ? state.playHistory : [tracks[newIndex]];
    const newPosition =
      state.playHistory.length > 0 ? state.historyPosition : 0;

    return {
      tracks,
      currentTrackIndex: newIndex,
      currentTrack: tracks[newIndex],
      playHistory: newHistory,
      historyPosition: newPosition,
    };
  };

  // Navigation helper functions
  const calculatePreviousIndex = (
    currentIndex: number,
    tracksLength: number,
  ): number => (currentIndex === 0 ? tracksLength - 1 : currentIndex - 1);

  const calculateNextIndex = (
    currentIndex: number,
    tracksLength: number,
  ): number => (currentIndex + 1) % tracksLength;

  const findTrackIndex = (tracks: Track[], targetTrack: Track): number =>
    tracks.findIndex((track) => track.id === targetTrack.id);

  const moveInHistory = (
    state: TrackStoreState,
    newPosition: number,
  ): Partial<TrackStoreState> => {
    const targetTrack = state.playHistory[newPosition];
    const targetIndex = findTrackIndex(state.tracks, targetTrack);

    if (targetIndex === -1) return {};

    return {
      currentTrackIndex: targetIndex,
      currentTrack: targetTrack,
      historyPosition: newPosition,
    };
  };

  const handleShuffleNext = (
    state: TrackStoreState,
  ): Partial<TrackStoreState> => {
    // Try to move forward in existing history first
    if (state.historyPosition < state.playHistory.length - 1) {
      return moveInHistory(state, state.historyPosition + 1);
    }

    // Generate new random track
    const availableTracks = state.tracks.filter(
      (_, i) => i !== state.currentTrackIndex,
    );
    if (availableTracks.length === 0) return {};

    const randomIndex = Math.floor(Math.random() * availableTracks.length);
    const selectedTrack = availableTracks[randomIndex];
    const newIndex = findTrackIndex(state.tracks, selectedTrack);

    const newHistory = [...state.playHistory];

    // Add current track to history if this is the first shuffle move
    if (state.currentTrackIndex !== null && state.historyPosition === -1) {
      newHistory.push(state.tracks[state.currentTrackIndex]);
    }

    newHistory.push(selectedTrack);

    return {
      currentTrackIndex: newIndex,
      currentTrack: selectedTrack,
      playHistory: newHistory,
      historyPosition: newHistory.length - 1,
    };
  };

  const handleShufflePrevious = (
    state: TrackStoreState,
  ): Partial<TrackStoreState> => {
    // Move backward in existing history
    if (state.historyPosition > 0) {
      return moveInHistory(state, state.historyPosition - 1);
    }

    // No history or only current track - fall back to sequential navigation
    const isMinimalHistory =
      state.playHistory.length === 0 ||
      (state.playHistory.length === 1 && state.historyPosition === 0);

    if (isMinimalHistory) {
      return handleSequentialPreviousWithHistory(state);
    }

    // At beginning of history - extend with sequential navigation
    return extendHistoryBackward(state);
  };

  const handleSequentialPreviousWithHistory = (
    state: TrackStoreState,
  ): Partial<TrackStoreState> => {
    if (state.currentTrackIndex === null) return {};

    const previousIndex = calculatePreviousIndex(
      state.currentTrackIndex,
      state.tracks.length,
    );
    const previousTrack = state.tracks[previousIndex];

    return {
      currentTrackIndex: previousIndex,
      currentTrack: previousTrack,
      playHistory: [previousTrack, state.currentTrack!],
      historyPosition: 0,
    };
  };

  const extendHistoryBackward = (
    state: TrackStoreState,
  ): Partial<TrackStoreState> => {
    if (state.currentTrackIndex === null) return {};

    const previousIndex = calculatePreviousIndex(
      state.currentTrackIndex,
      state.tracks.length,
    );
    const previousTrack = state.tracks[previousIndex];

    return {
      currentTrackIndex: previousIndex,
      currentTrack: previousTrack,
      playHistory: [previousTrack, ...state.playHistory],
      historyPosition: 0,
    };
  };

  /**
   * Clears shuffle history when disabling shuffle mode to ensure clean state.
   */
  const clearShuffleHistory = (
    state: TrackStoreState,
    newShuffleState: boolean,
  ): Partial<TrackStoreState> => {
    const isDisablingShuffle = state.is_shuffle && !newShuffleState;

    if (isDisablingShuffle && state.currentTrack) {
      return {
        is_shuffle: newShuffleState,
        playHistory: [state.currentTrack],
        historyPosition: 0,
      };
    }

    return { is_shuffle: newShuffleState };
  };

  return {
    subscribe,
    setTracks: (tracks: Track[]) =>
      update((state) => {
        if (tracks.length === 0) {
          return { ...state, ...createEmptyState(tracks) };
        }

        // Try to preserve current track if it exists in new tracks
        if (state.currentTrackIndex !== null && state.tracks.length > 0) {
          const currentTrack = state.tracks[state.currentTrackIndex];
          const newIndex = tracks.findIndex(
            (track) => track.id === currentTrack.id,
          );
          if (newIndex !== -1) {
            return {
              ...state,
              ...preserveCurrentTrack(state, tracks, newIndex),
            };
          }
        }

        // Only auto-select first track if no current track exists
        // This eliminates the need for skipAutoSelect parameter
        if (state.currentTrackIndex === null) {
          return { ...state, ...createInitialTrackState(tracks, 0) };
        }

        // If we had a current track but it's not in new tracks, clear selection
        return { ...state, ...createEmptyState(tracks) };
      }),
    setCurrentTrackIndex: (index: number | null) => {
      update((state) => {
        if (
          index === null ||
          state.tracks.length === 0 ||
          index < 0 ||
          index >= state.tracks.length
        ) {
          return { ...state, currentTrackIndex: null, currentTrack: null };
        }
        const track = state.tracks[index];

        // If history is empty (page initialization), initialize with current track
        if (state.playHistory.length === 0) {
          return {
            ...state,
            currentTrackIndex: index,
            currentTrack: track,
            playHistory: [track],
            historyPosition: 0,
          };
        }

        // If history has only one track and it's the auto-selected first track,
        // and we're setting a different track (backend restoration), replace it
        if (
          state.playHistory.length === 1 &&
          state.historyPosition === 0 &&
          state.currentTrackIndex === 0 &&
          index !== 0
        ) {
          return {
            ...state,
            currentTrackIndex: index,
            currentTrack: track,
            playHistory: [track],
            historyPosition: 0,
          };
        }

        // Check if the track is already in history
        const existingIndex = state.playHistory.findIndex(
          (t) => t && t.id === track.id,
        );
        if (existingIndex !== -1) {
          // Track exists in history, just update position to point to it
          return {
            ...state,
            currentTrackIndex: index,
            currentTrack: track,
            historyPosition: existingIndex,
          };
        }

        // Track not in history, add it and point to it
        const newHistory = [...state.playHistory, track];
        return {
          ...state,
          currentTrackIndex: index,
          currentTrack: track,
          playHistory: newHistory,
          historyPosition: newHistory.length - 1,
        };
      });
    },
    playTrack: (index: number) => {
      update((state) => {
        if (
          index < 0 ||
          index >= state.tracks.length ||
          state.currentTrackIndex === index
        ) {
          return state;
        }

        const track = state.tracks[index];

        return {
          ...state,
          currentTrackIndex: index,
          currentTrack: track,
          playHistory: [track],
          historyPosition: 0,
        };
      });
    },
    nextTrack: () => {
      update((state) => {
        if (state.currentTrackIndex === null || state.tracks.length === 0) {
          return state;
        }

        if (state.is_shuffle) {
          const shuffleResult = handleShuffleNext(state);
          return shuffleResult ? { ...state, ...shuffleResult } : state;
        }

        const nextIndex = calculateNextIndex(
          state.currentTrackIndex,
          state.tracks.length,
        );
        return {
          ...state,
          currentTrackIndex: nextIndex,
          currentTrack: state.tracks[nextIndex],
        };
      });
    },
    previousTrack: () => {
      update((state) => {
        if (state.currentTrackIndex === null || state.tracks.length === 0) {
          return state;
        }

        if (state.is_shuffle) {
          const shuffleResult = handleShufflePrevious(state);
          return shuffleResult ? { ...state, ...shuffleResult } : state;
        }

        const previousIndex = calculatePreviousIndex(
          state.currentTrackIndex,
          state.tracks.length,
        );
        return {
          ...state,
          currentTrackIndex: previousIndex,
          currentTrack: state.tracks[previousIndex],
        };
      });
    },
    toggleShuffle: () =>
      update((state) => {
        const newShuffleState = !state.is_shuffle;
        return { ...state, ...clearShuffleHistory(state, newShuffleState) };
      }),
    setShuffle: (is_shuffle: boolean) =>
      update((state) => ({
        ...state,
        ...clearShuffleHistory(state, is_shuffle),
      })),
    reset: () => set(initialState),
  };
}

export const trackStore = createTrackStore();
