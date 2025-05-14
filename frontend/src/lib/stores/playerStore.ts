import { writable } from 'svelte/store';
import type { Track } from '$lib/types';

export interface PlayerStoreState {
	currentTrack: Track | null;
	isPlaying: boolean;
	currentTime: number; // in seconds
	duration: number; // in seconds
	volume: number; // 0-1
	isMuted: boolean;
}

const initialState: PlayerStoreState = {
	currentTrack: null,
	isPlaying: false,
	currentTime: 0,
	duration: 0,
	volume: 1.0,
	isMuted: false
};

function createPlayerStore() {
	const { subscribe, set, update } = writable<PlayerStoreState>(initialState);

	return {
		subscribe,
		setTrack: (track: Track) =>
			update((state) => ({ ...state, currentTrack: track, duration: track.duration })),
		play: () => update((state) => ({ ...state, isPlaying: true })),
		pause: () => update((state) => ({ ...state, isPlaying: false })),
		togglePlayPause: () => update((state) => ({ ...state, isPlaying: !state.isPlaying })),
		setCurrentTime: (time: number) => update((state) => ({ ...state, currentTime: time })),
		setVolume: (volume: number) =>
			update((state) => ({ ...state, volume: Math.max(0, Math.min(1, volume)) })),
		toggleMute: () => update((state) => ({ ...state, isMuted: !state.isMuted })),
		reset: () => set(initialState)
	};
}

export const playerStore = createPlayerStore();
