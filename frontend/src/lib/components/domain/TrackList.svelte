<script lang="ts">
  import type { Track } from "$lib/types";
  import TrackItem from "./TrackItem.svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { browser } from "$app/environment";
  import { onMount, onDestroy } from "svelte";

  export let tracks: Track[] = [];

  let lastMouseMoveTime = Date.now();

  function handleMouseMove() {
    lastMouseMoveTime = Date.now();
  }

  onMount(() => {
    window.addEventListener("mousemove", handleMouseMove);
  });

  onDestroy(() => {
    window.removeEventListener("mousemove", handleMouseMove);
  });

  $: currentTrackIndex = $trackStore.currentTrackIndex;

  // Auto-scroll only if user hasn't moved mouse in last 5 seconds
  $: if (browser && currentTrackIndex !== null && tracks.length > 0) {
    const currentTrack = tracks[currentTrackIndex];
    if (currentTrack) {
      setTimeout(() => {
        if (Date.now() - lastMouseMoveTime > 5000) {
          const trackElement = document.getElementById(
            `track-item-${currentTrack.id}`,
          );
          if (trackElement) {
            trackElement.scrollIntoView({
              behavior: "smooth",
              block: "nearest",
            });
          }
        }
      }, 100); // Small delay to ensure DOM is updated
    }
  }
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
