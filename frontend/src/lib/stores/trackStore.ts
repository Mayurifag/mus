import { writable } from "svelte/store";
import type { Track } from "$lib/types";
import {
  calculateNextIndex,
  calculatePreviousIndex,
  clearShuffleHistory,
  handleShuffleNext,
  handleShufflePrevious,
} from "$lib/stores/trackNavigation";

export interface TrackStoreState {
  tracks: Track[];
  currentTrackIndex: number | null;
  playHistory: Track[];
  historyPosition: number;
  currentTrack: Track | null;
  is_shuffle: boolean;
  selectedArtist: string | null;
}

const initialState: TrackStoreState = {
  tracks: [],
  currentTrackIndex: null,
  playHistory: [],
  historyPosition: -1,
  currentTrack: null,
  is_shuffle: false,
  selectedArtist: null,
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
    setArtistFilter: (artist: string) =>
      update((state) => ({
        ...state,
        selectedArtist: artist,
      })),
    clearArtistFilter: () =>
      update((state) => ({
        ...state,
        selectedArtist: null,
      })),
    addTrack: (track: Track) =>
      update((state) => {
        const newTracks = [track, ...state.tracks];
        const newCurrentTrackIndex =
          state.currentTrackIndex !== null ? state.currentTrackIndex + 1 : null;
        return {
          ...state,
          tracks: newTracks,
          currentTrackIndex: newCurrentTrackIndex,
        };
      }),
    updateTrack: (updatedTrack: Track) =>
      update((state) => {
        const trackIndex = state.tracks.findIndex(
          (t) => t.id === updatedTrack.id,
        );
        if (trackIndex === -1) return state;

        const newTracks = [...state.tracks];
        newTracks[trackIndex] = updatedTrack;

        const newCurrentTrack =
          state.currentTrack?.id === updatedTrack.id
            ? updatedTrack
            : state.currentTrack;

        const newPlayHistory = state.playHistory.map((t) =>
          t.id === updatedTrack.id ? updatedTrack : t,
        );

        return {
          ...state,
          tracks: newTracks,
          currentTrack: newCurrentTrack,
          playHistory: newPlayHistory,
        };
      }),
    deleteTrack: (trackId: number) => {
      return update((state) => {
        const trackIndex = state.tracks.findIndex((t) => t.id === trackId);
        if (trackIndex === -1) return state;

        const newTracks = state.tracks.filter((t) => t.id !== trackId);
        let newCurrentTrackIndex = state.currentTrackIndex;
        let newCurrentTrack = state.currentTrack;

        if (state.currentTrackIndex !== null) {
          if (trackIndex < state.currentTrackIndex) {
            newCurrentTrackIndex = state.currentTrackIndex - 1;
          } else if (trackIndex === state.currentTrackIndex) {
            if (newTracks.length === 0) {
              newCurrentTrackIndex = null;
              newCurrentTrack = null;
            } else if (trackIndex < newTracks.length) {
              newCurrentTrack = newTracks[trackIndex];
            } else {
              newCurrentTrackIndex = newTracks.length - 1;
              newCurrentTrack = newTracks[newCurrentTrackIndex];
            }
          }
        }

        const deletedTrackHistoryIndex = state.playHistory.findIndex(
          (t) => t.id === trackId,
        );
        const newPlayHistory = state.playHistory.filter(
          (t) => t.id !== trackId,
        );
        let newHistoryPosition = state.historyPosition;

        if (
          deletedTrackHistoryIndex !== -1 &&
          deletedTrackHistoryIndex < state.historyPosition
        ) {
          newHistoryPosition--;
        } else if (
          deletedTrackHistoryIndex !== -1 &&
          deletedTrackHistoryIndex === state.historyPosition
        ) {
          newHistoryPosition = newPlayHistory.length - 1;
        }

        return {
          ...state,
          tracks: newTracks,
          currentTrackIndex: newCurrentTrackIndex,
          currentTrack: newCurrentTrack,
          playHistory: newPlayHistory,
          historyPosition: newHistoryPosition,
        };
      });
    },
    reset: () => set(initialState),
  };
}

export const trackStore = createTrackStore();
