import { writable, get } from "svelte/store";
import type { Track } from "$lib/types";
import { playerStore } from "./playerStore";

export interface TrackStoreState {
  tracks: Track[];
  currentTrackIndex: number | null;
  playHistory: Track[];
  isLoading: boolean;
}

const initialState: TrackStoreState = {
  tracks: [],
  currentTrackIndex: null,
  playHistory: [],
  isLoading: true,
};

function createTrackStore() {
  const { subscribe, set, update } = writable<TrackStoreState>(initialState);

  return {
    subscribe,
    setTracks: (tracks: Track[]) =>
      update((state) => {
        // Try to find the same track ID in the new list if there's a current track
        if (state.currentTrackIndex !== null && state.tracks.length > 0) {
          const currentTrack = state.tracks[state.currentTrackIndex];
          const newIndex = tracks.findIndex(
            (track) => track.id === currentTrack.id,
          );
          if (newIndex !== -1) {
            return {
              ...state,
              tracks,
              currentTrackIndex: newIndex,
              isLoading: false,
            };
          }
        }
        return { ...state, tracks, isLoading: false };
      }),
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
          // Add current track to history if it exists
          const newHistory = [...state.playHistory];
          if (
            state.currentTrackIndex !== null &&
            state.tracks[state.currentTrackIndex]
          ) {
            newHistory.push(state.tracks[state.currentTrackIndex]);
          }

          playerStore.setTrack(state.tracks[index]);
          playerStore.play();
          return {
            ...state,
            currentTrackIndex: index,
            playHistory: newHistory,
          };
        }
        return state;
      });
    },
    nextTrack: () => {
      update((state) => {
        if (state.currentTrackIndex === null || state.tracks.length === 0) {
          return state;
        }

        // Handle shuffle mode
        if (get(playerStore).is_shuffle) {
          // Get a random track that's not the current one
          const availableTracks = state.tracks.filter(
            (_, i) => i !== state.currentTrackIndex,
          );
          if (availableTracks.length === 0) {
            return state; // Only one track in the list
          }

          const randomIndex = Math.floor(
            Math.random() * availableTracks.length,
          );
          const selectedTrack = availableTracks[randomIndex];
          const newIndex = state.tracks.findIndex(
            (track) => track.id === selectedTrack.id,
          );

          // Add current track to history
          const newHistory = [...state.playHistory];
          if (state.currentTrackIndex !== null) {
            newHistory.push(state.tracks[state.currentTrackIndex]);
          }

          playerStore.setTrack(state.tracks[newIndex]);
          playerStore.play();
          return {
            ...state,
            currentTrackIndex: newIndex,
            playHistory: newHistory,
          };
        } else {
          // Standard sequential play
          const nextIndex = (state.currentTrackIndex + 1) % state.tracks.length;
          playerStore.setTrack(state.tracks[nextIndex]);
          playerStore.play();
          return { ...state, currentTrackIndex: nextIndex };
        }
      });
    },
    previousTrack: () => {
      update((state) => {
        if (state.currentTrackIndex === null || state.tracks.length === 0) {
          return state;
        }

        // Handle shuffle mode with history
        if (get(playerStore).is_shuffle && state.playHistory.length > 0) {
          // Pop the last track from history
          const newHistory = [...state.playHistory];
          const previousTrack = newHistory.pop();

          if (previousTrack) {
            const previousIndex = state.tracks.findIndex(
              (track) => track.id === previousTrack.id,
            );

            if (previousIndex !== -1) {
              playerStore.setTrack(state.tracks[previousIndex]);
              playerStore.play();
              return {
                ...state,
                currentTrackIndex: previousIndex,
                playHistory: newHistory,
              };
            }
          }
        }

        // Default behavior (no shuffle or empty history)
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
