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

export interface TrackState {
  tracks: Track[];
  tracksLoading: boolean;
  currentTrackId: string | null;
  playHistoryIds: string[];
  historyPosition: number;
  is_shuffle: boolean;
  selectedArtist: string | null;
  selectedCategory: string | null;
  playRequestId: number;
}

export interface TrackStoreState extends TrackState {
  readonly currentTrackIndex: number | null;
  readonly currentTrack: Track | null;
  readonly playHistory: Track[];
}

const initialState: TrackState = {
  tracks: [],
  tracksLoading: true,
  currentTrackId: null,
  playHistoryIds: [],
  historyPosition: -1,
  is_shuffle: false,
  selectedArtist: null,
  selectedCategory: null,
  playRequestId: 0,
};

function replaceTrack(
  state: TrackState,
  track: Track,
  previousId: string,
): TrackState {
  return {
    ...state,
    tracks: state.tracks.map((entry) =>
      entry.id === previousId ? track : entry,
    ),
    currentTrackId:
      state.currentTrackId === previousId ? track.id : state.currentTrackId,
    playHistoryIds: state.playHistoryIds.map((id) =>
      id === previousId ? track.id : id,
    ),
  };
}

function createTrackStore() {
  const store = writable<TrackState>(initialState);
  const { set, update } = store;
  const view = derived(store, (state): TrackStoreState => {
    const index = state.tracks.findIndex(
      (track) => track.id === state.currentTrackId,
    );
    const byId = new Map(state.tracks.map((track) => [track.id, track]));
    return {
      ...state,
      currentTrackIndex: index === -1 ? null : index,
      currentTrack: state.tracks[index] ?? null,
      playHistory: state.playHistoryIds
        .map((id) => byId.get(id))
        .filter((track): track is Track => !!track),
    };
  });
  const emptySelection = {
    currentTrackId: null,
    playHistoryIds: [],
    historyPosition: -1,
  };

  return {
    subscribe: view.subscribe,
    setTracks: (tracks: Track[]) =>
      update((state) => {
        const next = { ...state, tracks, tracksLoading: false };
        if (tracks.length === 0) return { ...next, ...emptySelection };
        if (state.currentTrackId === null) {
          return {
            ...next,
            currentTrackId: tracks[0].id,
            playHistoryIds: [tracks[0].id],
            historyPosition: 0,
          };
        }
        const ids = new Set(tracks.map((track) => track.id));
        if (!ids.has(state.currentTrackId))
          return { ...next, ...emptySelection };
        let position = state.historyPosition;
        const history = state.playHistoryIds.filter((id, index) => {
          if (ids.has(id)) return true;
          if (index < state.historyPosition) position--;
          return false;
        });
        return {
          ...next,
          playHistoryIds: history.length ? history : [state.currentTrackId],
          historyPosition: history.length ? position : 0,
        };
      }),
    setCurrentTrackIndex: (index: number | null) =>
      update((state) => {
        const track = index === null ? undefined : state.tracks[index];
        if (!track) return { ...state, currentTrackId: null };
        if (
          state.playHistoryIds.length === 0 ||
          (state.playHistoryIds.length === 1 &&
            state.historyPosition === 0 &&
            state.currentTrackId === state.tracks[0].id &&
            index !== 0)
        ) {
          return {
            ...state,
            currentTrackId: track.id,
            playHistoryIds: [track.id],
            historyPosition: 0,
          };
        }
        const position = state.playHistoryIds.indexOf(track.id);
        if (position !== -1)
          return {
            ...state,
            currentTrackId: track.id,
            historyPosition: position,
          };
        return {
          ...state,
          currentTrackId: track.id,
          playHistoryIds: [...state.playHistoryIds, track.id],
          historyPosition: state.playHistoryIds.length,
          playRequestId: state.playRequestId + 1,
        };
      }),
    playTrack: (index: number) =>
      update((state) => {
        const track = state.tracks[index];
        if (!track || state.currentTrackId === track.id) return state;
        return {
          ...state,
          currentTrackId: track.id,
          playHistoryIds: [track.id],
          historyPosition: 0,
          playRequestId: state.playRequestId + 1,
        };
      }),
    nextTrack: (play = false) =>
      update((state) => {
        if (state.tracks.length === 0) return state;
        const nextState = (partial: Partial<TrackState>) => ({
          ...state,
          ...partial,
          playRequestId:
            state.playRequestId +
            Number(
              play &&
                partial.currentTrackId !== undefined &&
                partial.currentTrackId !== state.currentTrackId,
            ),
        });
        if (state.selectedArtist || state.selectedCategory) {
          const indexes = selectedTrackIndexes(state);
          return indexes.length
            ? nextState(navigateFilteredTracks(state, indexes, 1))
            : state;
        }
        if (state.currentTrackId === null) return state;
        if (state.is_shuffle) return nextState(handleShuffleNext(state));
        const index = state.tracks.findIndex(
          (track) => track.id === state.currentTrackId,
        );
        return nextState({
          currentTrackId:
            state.tracks[calculateNextIndex(index, state.tracks.length)].id,
        });
      }),
    previousTrack: () =>
      update((state) => {
        if (state.tracks.length === 0) return state;
        if (state.selectedArtist || state.selectedCategory) {
          const indexes = selectedTrackIndexes(state);
          return indexes.length
            ? { ...state, ...navigateFilteredTracks(state, indexes, -1) }
            : state;
        }
        if (state.currentTrackId === null) return state;
        if (state.is_shuffle)
          return { ...state, ...handleShufflePrevious(state) };
        const index = state.tracks.findIndex(
          (track) => track.id === state.currentTrackId,
        );
        return {
          ...state,
          currentTrackId:
            state.tracks[calculatePreviousIndex(index, state.tracks.length)].id,
        };
      }),
    toggleShuffle: () =>
      update((state) => ({
        ...state,
        ...clearShuffleHistory(state, !state.is_shuffle),
      })),
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
      update((state) => ({ ...state, selectedArtist: null })),
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
      update((state) => ({ ...state, selectedCategory: null })),
    addTrack: (track: Track) =>
      update((state) =>
        state.tracks.some((entry) => entry.id === track.id)
          ? replaceTrack(state, track, track.id)
          : { ...state, tracks: [track, ...state.tracks] },
      ),
    updateTrack: (track: Track, previousId = track.id) =>
      update((state) =>
        state.tracks.some((entry) => entry.id === previousId)
          ? replaceTrack(state, track, previousId)
          : state,
      ),
    deleteTrack: (trackId: string) =>
      update((state) => {
        const index = state.tracks.findIndex((track) => track.id === trackId);
        if (index === -1) return state;
        const tracks = state.tracks.filter((track) => track.id !== trackId);
        const currentTrackId =
          state.currentTrackId === trackId
            ? (tracks[Math.min(index, tracks.length - 1)]?.id ?? null)
            : state.currentTrackId;
        let position = state.historyPosition;
        const history = state.playHistoryIds.filter((id, index) => {
          if (id !== trackId) return true;
          if (index < state.historyPosition) position--;
          return false;
        });
        position = Math.min(position, history.length - 1);
        if (state.currentTrackId === trackId && currentTrackId) {
          position = history.indexOf(currentTrackId);
          if (position === -1) {
            position = history.length;
            history.push(currentTrackId);
          }
        }
        return {
          ...state,
          tracks,
          currentTrackId,
          playHistoryIds: history,
          historyPosition: position,
        };
      }),
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
