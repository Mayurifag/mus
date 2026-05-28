import { writable } from "svelte/store";
import type { Track } from "$lib/types";
import {
  calculateNextIndex,
  calculatePreviousIndex,
  clearShuffleHistory,
  handleShuffleNext,
  handleShufflePrevious,
} from "$lib/stores/trackNavigation";
import { parseArtists } from "$lib/utils/formatters";

export interface TrackStoreState {
  tracks: Track[];
  currentTrackIndex: number | null;
  playHistory: Track[];
  historyPosition: number;
  currentTrack: Track | null;
  is_shuffle: boolean;
  selectedArtist: string | null;
  playRequestId: number;
  shuffleLookahead: ShuffleLookahead | null;
}

export interface ShuffleLookahead {
  track: Track;
  currentTrackId: number;
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
  playRequestId: 0,
  shuffleLookahead: null,
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
    shuffleLookahead: null,
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
    shuffleLookahead: null,
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
      shuffleLookahead: null,
    };
  };

  const trackMatchesArtist = (track: Track, artist: string): boolean =>
    parseArtists(track.artist).includes(artist);

  const getSelectedArtistTrackIndexes = (state: TrackStoreState): number[] => {
    const selectedArtist = state.selectedArtist;
    if (!selectedArtist) return [];

    return state.tracks.reduce<number[]>((indexes, track, index) => {
      if (trackMatchesArtist(track, selectedArtist)) {
        indexes.push(index);
      }
      return indexes;
    }, []);
  };

  const findScopedTrackIndex = (
    state: TrackStoreState,
    scopedIndexes: number[],
    targetTrack: Track,
  ): number =>
    scopedIndexes.find(
      (trackIndex) => state.tracks[trackIndex].id === targetTrack.id,
    ) ?? -1;

  const getScopedHistoryMove = (
    state: TrackStoreState,
    scopedIndexes: number[],
    startPosition: number,
    direction: 1 | -1,
  ): { trackIndex: number; historyPosition: number } | null => {
    for (
      let position = startPosition;
      position >= 0 && position < state.playHistory.length;
      position += direction
    ) {
      const trackIndex = findScopedTrackIndex(
        state,
        scopedIndexes,
        state.playHistory[position],
      );
      if (trackIndex !== -1) {
        return { trackIndex, historyPosition: position };
      }
    }

    return null;
  };

  const getSequentialScopedTrackIndex = (
    state: TrackStoreState,
    scopedIndexes: number[],
    direction: 1 | -1,
  ): number => {
    const currentPosition =
      state.currentTrackIndex === null
        ? -1
        : scopedIndexes.indexOf(state.currentTrackIndex);
    const scopedPosition =
      currentPosition === -1
        ? direction === 1
          ? 0
          : scopedIndexes.length - 1
        : (currentPosition + direction + scopedIndexes.length) %
          scopedIndexes.length;

    return scopedIndexes[scopedPosition];
  };

  const getRandomScopedTrackIndex = (
    state: TrackStoreState,
    scopedIndexes: number[],
    lookahead?: Track | null,
  ): number => {
    const lookaheadIndex = lookahead
      ? findScopedTrackIndex(state, scopedIndexes, lookahead)
      : -1;
    if (lookaheadIndex !== -1 && lookaheadIndex !== state.currentTrackIndex) {
      return lookaheadIndex;
    }

    const currentInScope =
      state.currentTrackIndex !== null &&
      scopedIndexes.includes(state.currentTrackIndex);
    const availableIndexes =
      currentInScope && scopedIndexes.length > 1
        ? scopedIndexes.filter(
            (trackIndex) => trackIndex !== state.currentTrackIndex,
          )
        : scopedIndexes;

    return availableIndexes[
      Math.floor(Math.random() * availableIndexes.length)
    ];
  };

  const navigateSelectedArtistTrack = (
    state: TrackStoreState,
    scopedIndexes: number[],
    direction: 1 | -1,
    lookahead?: Track | null,
  ): Partial<TrackStoreState> => {
    if (state.is_shuffle) {
      const historyMove = getScopedHistoryMove(
        state,
        scopedIndexes,
        state.historyPosition + direction,
        direction,
      );

      if (historyMove) {
        return {
          currentTrackIndex: historyMove.trackIndex,
          currentTrack: state.tracks[historyMove.trackIndex],
          historyPosition: historyMove.historyPosition,
        };
      }
    }

    const trackIndex =
      state.is_shuffle && direction === 1
        ? getRandomScopedTrackIndex(state, scopedIndexes, lookahead)
        : getSequentialScopedTrackIndex(state, scopedIndexes, direction);
    const track = state.tracks[trackIndex];

    if (!state.is_shuffle) {
      return {
        currentTrackIndex: trackIndex,
        currentTrack: track,
      };
    }

    const currentInScope =
      state.currentTrackIndex !== null &&
      scopedIndexes.includes(state.currentTrackIndex);
    if (direction === 1 && currentInScope && scopedIndexes.length === 1) {
      return {};
    }

    const scopedHistory = state.playHistory
      .slice(0, Math.max(state.historyPosition + 1, 0))
      .filter(
        (track) => findScopedTrackIndex(state, scopedIndexes, track) !== -1,
      );

    if (scopedHistory.length === 0 && currentInScope && state.currentTrack) {
      scopedHistory.push(state.currentTrack);
    }

    const playHistory =
      direction === 1 ? [...scopedHistory, track] : [track, ...scopedHistory];

    return {
      currentTrackIndex: trackIndex,
      currentTrack: track,
      playHistory,
      historyPosition: direction === 1 ? playHistory.length - 1 : 0,
    };
  };

  const getValidShuffleLookahead = (state: TrackStoreState): Track | null => {
    const lookahead = state.shuffleLookahead;
    if (!lookahead || !state.currentTrack) return null;
    if (lookahead.currentTrackId !== state.currentTrack.id) return null;
    if (lookahead.selectedArtist !== state.selectedArtist) return null;

    return lookahead.track;
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
          return {
            ...state,
            currentTrackIndex: null,
            currentTrack: null,
            shuffleLookahead: null,
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
            shuffleLookahead: null,
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
            shuffleLookahead: null,
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
            shuffleLookahead: null,
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
          shuffleLookahead: null,
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
          shuffleLookahead: null,
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

        const lookahead = getValidShuffleLookahead(state);

        if (state.selectedArtist) {
          const scopedIndexes = getSelectedArtistTrackIndexes(state);
          if (scopedIndexes.length === 0) return state;

          return nextState({
            ...navigateSelectedArtistTrack(state, scopedIndexes, 1, lookahead),
            shuffleLookahead: null,
          });
        }

        if (state.currentTrackIndex === null) {
          return state;
        }

        if (state.is_shuffle) {
          const shuffleResult = handleShuffleNext(state, lookahead);
          return shuffleResult
            ? nextState({ ...shuffleResult, shuffleLookahead: null })
            : { ...state, shuffleLookahead: null };
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

        if (state.selectedArtist) {
          const scopedIndexes = getSelectedArtistTrackIndexes(state);
          if (scopedIndexes.length === 0) return state;

          return {
            ...state,
            ...navigateSelectedArtistTrack(state, scopedIndexes, -1),
            shuffleLookahead: null,
          };
        }

        if (state.currentTrackIndex === null) {
          return state;
        }

        if (state.is_shuffle) {
          const shuffleResult = handleShufflePrevious(state);
          return shuffleResult
            ? { ...state, ...shuffleResult, shuffleLookahead: null }
            : { ...state, shuffleLookahead: null };
        }

        const previousIndex = calculatePreviousIndex(
          state.currentTrackIndex,
          state.tracks.length,
        );
        return {
          ...state,
          currentTrackIndex: previousIndex,
          currentTrack: state.tracks[previousIndex],
          shuffleLookahead: null,
        };
      });
    },
    toggleShuffle: () =>
      update((state) => {
        const newShuffleState = !state.is_shuffle;
        return {
          ...state,
          ...clearShuffleHistory(state, newShuffleState),
          shuffleLookahead: null,
        };
      }),
    setShuffle: (is_shuffle: boolean) =>
      update((state) => ({
        ...state,
        ...clearShuffleHistory(state, is_shuffle),
        shuffleLookahead: null,
      })),
    setArtistFilter: (artist: string) =>
      update((state) =>
        state.selectedArtist === artist
          ? state
          : {
              ...state,
              selectedArtist: artist,
              shuffleLookahead: null,
            },
      ),
    clearArtistFilter: () =>
      update((state) => ({
        ...state,
        selectedArtist: null,
        shuffleLookahead: null,
      })),
    setShuffleLookahead: (
      track: Track | null,
      currentTrackId: number | null,
      selectedArtist: string | null,
    ) =>
      update((state) => {
        if (
          !state.is_shuffle ||
          !state.currentTrack ||
          state.currentTrack.id !== currentTrackId ||
          state.selectedArtist !== selectedArtist
        ) {
          return state;
        }

        if (!track) {
          return { ...state, shuffleLookahead: null };
        }

        if (!state.tracks.some((item) => item.id === track.id)) {
          return state;
        }

        return {
          ...state,
          shuffleLookahead: { track, currentTrackId, selectedArtist },
        };
      }),
    addTrack: (track: Track) =>
      update((state) => {
        const existingIndex = state.tracks.findIndex((t) => t.id === track.id);
        if (existingIndex !== -1) {
          const newTracks = [...state.tracks];
          newTracks[existingIndex] = track;
          const newCurrentTrack =
            state.currentTrack?.id === track.id ? track : state.currentTrack;
          const newPlayHistory = state.playHistory.map((t) =>
            t.id === track.id ? track : t,
          );

          return {
            ...state,
            tracks: newTracks,
            currentTrack: newCurrentTrack,
            playHistory: newPlayHistory,
          };
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
          shuffleLookahead: null,
        };
      });
    },
    reset: () => set(initialState),
  };
}

export const trackStore = createTrackStore();
