import { writable } from "svelte/store";
import type { Track } from "$lib/types";
import { playerStore } from "./playerStore";

export interface TrackStoreState {
  tracks: Track[];
  currentTrackIndex: number | null;
}

const initialState: TrackStoreState = {
  tracks: [],
  currentTrackIndex: null,
};

function createTrackStore() {
  const { subscribe, set, update } = writable<TrackStoreState>(initialState);

  return {
    subscribe,
    setTracks: (tracks: Track[]) => update((state) => ({ ...state, tracks })),
    setCurrentTrackIndex: (index: number | null) => {
      update((state) => {
        if (index !== null && state.tracks[index]) {
          playerStore.setTrack(state.tracks[index]);
          return { ...state, currentTrackIndex: index };
        }
        return { ...state, currentTrackIndex: null };
      });
    },
    playTrack: (index: number) => {
      update((state) => {
        if (index >= 0 && index < state.tracks.length) {
          playerStore.setTrack(state.tracks[index]);
          playerStore.play();
          return { ...state, currentTrackIndex: index };
        }
        return state;
      });
    },
    nextTrack: () => {
      update((state) => {
        if (state.currentTrackIndex === null || state.tracks.length === 0) {
          return state;
        }

        const nextIndex = (state.currentTrackIndex + 1) % state.tracks.length;
        playerStore.setTrack(state.tracks[nextIndex]);
        playerStore.play();
        return { ...state, currentTrackIndex: nextIndex };
      });
    },
    previousTrack: () => {
      update((state) => {
        if (state.currentTrackIndex === null || state.tracks.length === 0) {
          return state;
        }

        const previousIndex =
          state.currentTrackIndex === 0
            ? state.tracks.length - 1
            : state.currentTrackIndex - 1;

        playerStore.setTrack(state.tracks[previousIndex]);
        playerStore.play();
        return { ...state, currentTrackIndex: previousIndex };
      });
    },
    reset: () => set(initialState),
  };
}

export const trackStore = createTrackStore();
