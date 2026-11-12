import { derived, writable } from "svelte/store";
import type { Track } from "$lib/types";
import {
  calculateNextIndex,
  calculatePreviousIndex,
  clearShuffleHistory,
  handleShuffleNext,
  handleShufflePrevious,
} from "$lib/stores/trackNavigation";
import { parseArtists } from "$lib/utils/formatters";
import { trackCategoryNames } from "$lib/utils/categoryMarkers";
import {
  navigateFilteredTracks,
  selectedTrackIndexes,
} from "$lib/stores/filteredTrackNavigation";

export interface TrackStoreState {
  tracks: Track[];
  tracksLoading: boolean;
  currentTrackIndex: number | null;
  playHistory: Track[];
  historyPosition: number;
  currentTrack: Track | null;
  is_shuffle: boolean;
  selectedArtist: string | null;
  selectedCategory: string | null;
  playRequestId: number;
}

const initialState: TrackStoreState = {
  tracks: [],
  tracksLoading: true,
  currentTrackIndex: null,
  playHistory: [],
  historyPosition: -1,
  currentTrack: null,
  is_shuffle: false,
  selectedArtist: null,
  selectedCategory: null,
  playRequestId: 0,
};

function replaceTrack(
  state: TrackStoreState,
  track: Track,
  previousId: string,
): TrackStoreState {
  return {
    ...state,
    tracks: state.tracks.map((entry) =>
      entry.id === previousId ? track : entry,
    ),
    currentTrack:
      state.currentTrack?.id === previousId ? track : state.currentTrack,
    playHistory: state.playHistory.map((entry) =>
      entry.id === previousId ? track : entry,
    ),
  };
}

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
    const tracksById = new Map(tracks.map((track) => [track.id, track]));
    let newPosition = state.historyPosition;
    const newHistory: Track[] = [];
    for (const [position, entry] of state.playHistory.entries()) {
      const track = tracksById.get(entry.id);
      if (track) newHistory.push(track);
      else if (position < state.historyPosition) newPosition--;
    }

    return {
      tracks,
      currentTrackIndex: newIndex,
      currentTrack: tracks[newIndex],
      playHistory: newHistory.length > 0 ? newHistory : [tracks[newIndex]],
      historyPosition: newHistory.length > 0 ? newPosition : 0,
    };
  };

  return {
    subscribe,
    setTracks: (tracks: Track[]) =>
      update((state) => {
        state = { ...state, tracksLoading: false };
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
          return {
            ...state,
            currentTrackIndex: null,
            currentTrack: null,
          };
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
          playRequestId: state.playRequestId + 1,
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
          playRequestId: state.playRequestId + 1,
        };
      });
    },
    nextTrack: (play = false) => {
      update((state) => {
        if (state.tracks.length === 0) {
          return state;
        }

        const nextState = (partial: Partial<TrackStoreState>) => {
          if (
            play &&
            partial.currentTrackIndex !== undefined &&
            partial.currentTrackIndex !== state.currentTrackIndex
          ) {
            return {
              ...state,
              ...partial,
              playRequestId: state.playRequestId + 1,
            };
          }
          return { ...state, ...partial };
        };

        if (state.selectedArtist || state.selectedCategory) {
          const scopedIndexes = selectedTrackIndexes(state);
          if (scopedIndexes.length === 0) return state;

          return nextState(navigateFilteredTracks(state, scopedIndexes, 1));
        }

        if (state.currentTrackIndex === null) {
          return state;
        }

        if (state.is_shuffle) {
          const shuffleResult = handleShuffleNext(state);
          return shuffleResult ? nextState(shuffleResult) : state;
        }

        const nextIndex = calculateNextIndex(
          state.currentTrackIndex,
          state.tracks.length,
        );
        return nextState({
          currentTrackIndex: nextIndex,
          currentTrack: state.tracks[nextIndex],
        });
      });
    },
    previousTrack: () => {
      update((state) => {
        if (state.tracks.length === 0) {
          return state;
        }

        if (state.selectedArtist || state.selectedCategory) {
          const scopedIndexes = selectedTrackIndexes(state);
          if (scopedIndexes.length === 0) return state;

          return {
            ...state,
            ...navigateFilteredTracks(state, scopedIndexes, -1),
          };
        }

        if (state.currentTrackIndex === null) {
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
        return {
          ...state,
          ...clearShuffleHistory(state, newShuffleState),
        };
      }),
    setShuffle: (is_shuffle: boolean) =>
      update((state) => ({
        ...state,
        ...clearShuffleHistory(state, is_shuffle),
      })),
    setArtistFilter: (artist: string) =>
      update((state) =>
        state.selectedArtist === artist
          ? state
          : {
              ...state,
              selectedArtist: artist,
              selectedCategory: null,
            },
      ),
    clearArtistFilter: () =>
      update((state) => ({
        ...state,
        selectedArtist: null,
      })),
    setCategoryFilter: (category: string) =>
      update((state) =>
        state.selectedCategory === category
          ? state
          : {
              ...state,
              selectedCategory: category,
              selectedArtist: null,
            },
      ),
    clearCategoryFilter: () =>
      update((state) => ({
        ...state,
        selectedCategory: null,
      })),
    addTrack: (track: Track) =>
      update((state) => {
        const existingIndex = state.tracks.findIndex((t) => t.id === track.id);
        if (existingIndex !== -1) {
          return replaceTrack(state, track, track.id);
        }

        const newTracks = [track, ...state.tracks];
        const newCurrentTrackIndex =
          state.currentTrackIndex !== null ? state.currentTrackIndex + 1 : null;
        return {
          ...state,
          tracks: newTracks,
          currentTrackIndex: newCurrentTrackIndex,
        };
      }),
    updateTrack: (updatedTrack: Track, previousId = updatedTrack.id) =>
      update((state) => {
        const trackIndex = state.tracks.findIndex((t) => t.id === previousId);
        if (trackIndex === -1) return state;

        return replaceTrack(state, updatedTrack, previousId);
      }),
    deleteTrack: (trackId: string) => {
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

        let newHistoryPosition = state.historyPosition;
        const newPlayHistory = state.playHistory.filter((track, position) => {
          if (track.id !== trackId) return true;
          if (position < state.historyPosition) newHistoryPosition--;
          return false;
        });
        newHistoryPosition = Math.min(
          newHistoryPosition,
          newPlayHistory.length - 1,
        );

        if (state.currentTrack?.id === trackId && newCurrentTrack) {
          const replacementId = newCurrentTrack.id;
          newHistoryPosition = newPlayHistory.findIndex(
            (track) => track.id === replacementId,
          );
          if (newHistoryPosition === -1) {
            newHistoryPosition = newPlayHistory.length;
            newPlayHistory.push(newCurrentTrack);
          }
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

export const artistCountsStore = derived(trackStore, ($trackStore) => {
  const counts = new Map<string, number>();

  for (const track of $trackStore.tracks) {
    for (const artist of new Set(parseArtists(track.artist))) {
      counts.set(artist, (counts.get(artist) ?? 0) + 1);
    }
  }

  return Object.fromEntries(counts);
});

export const categoryCountsStore = derived(trackStore, ($trackStore) => {
  const counts = new Map<string, number>();

  for (const track of $trackStore.tracks) {
    for (const category of trackCategoryNames(track)) {
      counts.set(category, (counts.get(category) ?? 0) + 1);
    }
  }

  return Object.fromEntries(counts);
});
