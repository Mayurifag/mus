import { writable } from "svelte/store";
import type { Track } from "$lib/types";

export interface TrackStoreState {
  tracks: Track[];
  currentTrackIndex: number | null;
  playHistory: Track[];
  currentTrack: Track | null;
  is_shuffle: boolean;
}

const initialState: TrackStoreState = {
  tracks: [],
  currentTrackIndex: null,
  playHistory: [],
  currentTrack: null,
  is_shuffle: false,
};

function createTrackStore() {
  const { subscribe, set, update } = writable<TrackStoreState>(initialState);

  return {
    subscribe,
    setTracks: (tracks: Track[]) =>
      update((state) => {
        // If tracks array is empty, set currentTrackIndex to null
        if (tracks.length === 0) {
          return {
            ...state,
            tracks,
            currentTrackIndex: null,
            currentTrack: null,
          };
        }

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
              currentTrack: tracks[newIndex],
            };
          }
        }

        // If no prior state or current track not found, default to first track (index 0)
        return {
          ...state,
          tracks,
          currentTrackIndex: 0,
          currentTrack: tracks[0],
        };
      }),
    setCurrentTrackIndex: (index: number | null) => {
      update((state) => {
        if (index === null || state.tracks.length === 0) {
          return { ...state, currentTrackIndex: null, currentTrack: null };
        }
        const track = state.tracks[index];
        return { ...state, currentTrackIndex: index, currentTrack: track };
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

          const track = state.tracks[index];
          return {
            ...state,
            currentTrackIndex: index,
            currentTrack: track,
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
        if (state.is_shuffle) {
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

          const track = state.tracks[newIndex];
          return {
            ...state,
            currentTrackIndex: newIndex,
            currentTrack: track,
            playHistory: newHistory,
          };
        } else {
          // Standard sequential play
          const nextIndex = (state.currentTrackIndex + 1) % state.tracks.length;
          const track = state.tracks[nextIndex];
          return {
            ...state,
            currentTrackIndex: nextIndex,
            currentTrack: track,
          };
        }
      });
    },
    previousTrack: () => {
      update((state) => {
        if (state.currentTrackIndex === null || state.tracks.length === 0) {
          return state;
        }

        // Handle shuffle mode with history
        if (state.is_shuffle && state.playHistory.length > 0) {
          // Pop the last track from history
          const newHistory = [...state.playHistory];
          const previousTrack = newHistory.pop();

          if (previousTrack) {
            const previousIndex = state.tracks.findIndex(
              (track) => track.id === previousTrack.id,
            );

            if (previousIndex !== -1) {
              const track = state.tracks[previousIndex];
              return {
                ...state,
                currentTrackIndex: previousIndex,
                currentTrack: track,
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

        const track = state.tracks[previousIndex];
        return {
          ...state,
          currentTrackIndex: previousIndex,
          currentTrack: track,
        };
      });
    },
    toggleShuffle: () =>
      update((state) => ({ ...state, is_shuffle: !state.is_shuffle })),
    setShuffle: (is_shuffle: boolean) =>
      update((state) => ({ ...state, is_shuffle })),
    reset: () => set(initialState),
  };
}

export const trackStore = createTrackStore();
