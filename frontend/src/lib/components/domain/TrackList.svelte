<script lang="ts">
  import type { AudioService } from "$lib/services/AudioService";
  import TrackItem from "./TrackItem.svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { browser } from "$app/environment";
  import { tick, onMount } from "svelte";

  let { audioService }: { audioService?: AudioService } = $props();

  const tracks = $derived($trackStore.tracks);
  let isCurrentTrackVisible = $state(false);
  let observer: IntersectionObserver;

  function scrollToCurrentTrack({ force = false } = {}) {
    if (!browser || !$trackStore.currentTrack) return;

    const element = document.getElementById(
      `track-item-${$trackStore.currentTrack.id}`,
    );
    if (element && (force || !isCurrentTrackVisible)) {
      element.scrollIntoView({ behavior: "auto", block: "center" });
    }
  }

  onMount(() => {
    if (!browser) return;

    observer = new IntersectionObserver(
      (entries) => {
        isCurrentTrackVisible = entries[0].isIntersecting;
      },
      {
        rootMargin: "-15% 0px -30% 0px",
      },
    );

    const handleForceScroll = () => {
      scrollToCurrentTrack({ force: true });
    };

    document.body.addEventListener("force-scroll", handleForceScroll);

    return () => {
      observer?.disconnect();
      document.body.removeEventListener("force-scroll", handleForceScroll);
    };
  });

  $effect(() => {
    if (!browser || !observer || $trackStore.currentTrackIndex === null) return;

    observer.disconnect();

    if ($trackStore.currentTrack) {
      const element = document.getElementById(
        `track-item-${$trackStore.currentTrack.id}`,
      );
      if (element) {
        observer.observe(element);
        tick().then(() => {
          scrollToCurrentTrack();
        });
      }
    }
  });
</script>

<div class="flex flex-col space-y-1" data-testid="track-list">
  {#if tracks.length === 0}
    <div class="flex h-32 w-full flex-col items-center justify-center">
      <p class="text-muted-foreground mb-2 text-center">No tracks available</p>
    </div>
  {:else}
    <div class="space-y-1">
      {#each tracks as track, i (track.id)}
        <TrackItem
          {track}
          index={i}
          isSelected={$trackStore.currentTrackIndex === i}
          {audioService}
        />
      {/each}
    </div>
  {/if}
</div>
