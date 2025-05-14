<script lang="ts">
	import { onMount } from 'svelte';
	import { trackStore } from '$lib/stores/trackStore';
	import { playerStore } from '$lib/stores/playerStore';
	import type { Track } from '$lib/types';

	export let data: {
		tracks: Track[];
		playerState: null | {
			current_track_id: number | null;
			progress_seconds: number;
			volume_level: number;
			is_muted: boolean;
		};
	};

	// Initialize stores with server-loaded data
	onMount(() => {
		// Initialize tracks
		if (data.tracks && data.tracks.length > 0) {
			trackStore.setTracks(data.tracks);
		}

		// Initialize player state if available
		if (data.playerState) {
			const { current_track_id, progress_seconds, volume_level, is_muted } = data.playerState;

			// Set volume and mute state
			playerStore.setVolume(volume_level);
			if (is_muted) {
				playerStore.toggleMute();
			}

			// Set current track if exists
			if (current_track_id !== null) {
				const trackIndex = data.tracks.findIndex((track: Track) => track.id === current_track_id);
				if (trackIndex >= 0) {
					trackStore.setCurrentTrackIndex(trackIndex);
					// Set progress
					playerStore.setCurrentTime(progress_seconds);
				}
			}
		}
	});
</script>

<slot />

<!-- Audio element will be added in a subsequent task -->
