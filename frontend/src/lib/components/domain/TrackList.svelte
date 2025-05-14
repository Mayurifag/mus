<script lang="ts">
	import type { Track } from '$lib/types';
	import TrackItem from './TrackItem.svelte';
	import { trackStore } from '$lib/stores/trackStore';

	export let tracks: Track[] = [];

	$: currentTrackIndex = $trackStore.currentTrackIndex;
</script>

<div class="flex flex-col space-y-1" data-testid="track-list">
	{#if tracks.length === 0}
		<div class="flex h-32 w-full flex-col items-center justify-center">
			<p class="text-muted-foreground mb-2 text-center">No tracks available</p>
			<p class="text-muted-foreground text-center text-sm">
				Try scanning your music library to add tracks
			</p>
		</div>
	{:else}
		<div class="max-h-[calc(100vh-12rem)] overflow-y-auto pr-2">
			{#each tracks as track, i (track.id)}
				<TrackItem {track} index={i} isSelected={currentTrackIndex === i} />
			{/each}
		</div>
	{/if}
</div>
