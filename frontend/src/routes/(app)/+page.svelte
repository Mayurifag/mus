<script lang="ts">
	import { trackStore } from '$lib/stores/trackStore';

	// Access the store with the $ syntax for reactivity
	$: trackCount = $trackStore.tracks.length;
</script>

<div class="container mx-auto p-4">
	<h1 class="mb-4 text-2xl font-bold">Music Library</h1>

	{#if trackCount > 0}
		<p class="mb-4">Found {trackCount} tracks in your library.</p>
	{:else}
		<p class="mb-4">No tracks found. Use the scan function to add tracks.</p>
	{/if}

	<!-- The track list and player footer components will be implemented in subsequent tasks -->
</div>
