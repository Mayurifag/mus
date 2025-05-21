<script lang="ts">
  import { trackStore } from "$lib/stores/trackStore";
  import TrackList from "$lib/components/domain/TrackList.svelte";

  // Access the store with the $ syntax for reactivity
  $: tracks = $trackStore.tracks;
  $: trackCount = tracks.length;
</script>

<div class="container mx-auto p-4 pb-24">
  <div class="mb-6">
    <h1 class="mb-1 text-2xl font-bold">Music Library</h1>
    {#if trackCount > 0}
      <p class="text-muted-foreground">
        Found {trackCount} tracks in your library.
      </p>
    {:else}
      <p class="text-muted-foreground">
        No tracks found. Library will update automatically when new tracks are
        detected.
      </p>
    {/if}
  </div>

  <TrackList {tracks} />
</div>
