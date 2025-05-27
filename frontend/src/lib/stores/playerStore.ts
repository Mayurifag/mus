import { writable } from "svelte/store";
import type { Track } from "$lib/types";

export interface PlayerStoreState {
  currentTrack: Track | null;
  isPlaying: boolean;
  currentTime: number; // in seconds
  duration: number; // in seconds
  volume: number; // 0-1
  isMuted: boolean;
  is_shuffle: boolean;
  is_repeat: boolean;
}

const initialState: PlayerStoreState = {
  currentTrack: null,
  isPlaying: false,
  currentTime: 0,
  duration: 0,
  volume: 1.0,
  isMuted: false,
  is_shuffle: false,
  is_repeat: false,
};

function createPlayerStore() {
  const { subscribe, set, update } = writable<PlayerStoreState>(initialState);

  return {
    subscribe,
    update,
    setTrack: (track: Track, preserveTime = false) =>
      update((state) => ({
        ...state,
        currentTrack: track,
        currentTime: preserveTime ? state.currentTime : 0,
        duration: track.duration,
      })),
    play: () => update((state) => ({ ...state, isPlaying: true })),
    pause: () => update((state) => ({ ...state, isPlaying: false })),
    togglePlayPause: () =>
      update((state) => ({ ...state, isPlaying: !state.isPlaying })),
    setCurrentTime: (time: number) =>
      update((state) => ({ ...state, currentTime: time })),
    setDuration: (duration: number) =>
      update((state) => ({ ...state, duration })),
    setVolume: (volume: number) =>
      update((state) => ({
        ...state,
        volume: Math.max(0, Math.min(1, volume)),
      })),
    toggleMute: () =>
      update((state) => ({ ...state, isMuted: !state.isMuted })),
    setMuted: (isMuted: boolean) => update((state) => ({ ...state, isMuted })),
    toggleShuffle: () =>
      update((state) => ({ ...state, is_shuffle: !state.is_shuffle })),
    toggleRepeat: () =>
      update((state) => ({ ...state, is_repeat: !state.is_repeat })),
    reset: () => set(initialState),
  };
}

export const playerStore = createPlayerStore();
