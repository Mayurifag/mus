<script lang="ts">
  import { tick } from "svelte";
  import type { AudioService } from "$lib/services/AudioService";
  import type { TimeRange } from "$lib/types";
  import TrackItem from "./TrackItem.svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { createWindowVirtualizer } from "@tanstack/svelte-virtual";
  import { browser } from "$app/environment";
  import { updateEffectStats } from "$lib/utils/monitoredEffect";

  let { audioService }: { audioService?: AudioService } = $props();

  const tracks = $derived($trackStore.tracks);

  // Audio playback state variables
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let bufferedRanges = $state<TimeRange[]>([]);

  let virtualizer = $state<ReturnType<typeof createWindowVirtualizer> | null>(
    null,
  );
  let initialScrollDone = false;

  // Subscribe to audio service stores
  $effect(() => {
    updateEffectStats("TrackList_AudioServiceSubscriptions");

    if (audioService) {
      const unsubscribers: (() => void)[] = [];

      unsubscribers.push(
        audioService.isPlayingStore.subscribe((playing) => {
          isPlaying = playing;
        }),
      );

      unsubscribers.push(
        audioService.currentTimeStore.subscribe((time) => {
          currentTime = time;
        }),
      );

      unsubscribers.push(
        audioService.durationStore.subscribe((value) => {
          duration = value;
        }),
      );

      unsubscribers.push(
        audioService.currentBufferedRangesStore.subscribe((ranges) => {
          bufferedRanges = ranges;
        }),
      );

      return () => unsubscribers.forEach((unsub) => unsub());
    }
  });

  $effect(() => {
    updateEffectStats("TrackList_VirtualizerSetup");

    // TODO: we probably can use array length or size from backend
    if (browser) {
      virtualizer = createWindowVirtualizer({
        count: tracks.length,
        estimateSize: () => 72,
        overscan: 500,
        getScrollElement: () => window,
      });
    }
  });

  $effect(() => {
    updateEffectStats("TrackList_InitialScroll");

    if (
      !initialScrollDone &&
      virtualizer &&
      $trackStore.currentTrackIndex !== null
    ) {
      tick().then(() => {
        if (virtualizer && $trackStore.currentTrackIndex !== null) {
          $virtualizer!.scrollToIndex($trackStore.currentTrackIndex, {
            align: "center",
          });
        }
      });
      initialScrollDone = true;
    }
  });

  function measureElement(node: HTMLElement) {
    if (virtualizer) {
      $virtualizer!.measureElement(node);
    }
  }
</script>

<div class="flex flex-col" data-testid="track-list">
  {#if tracks.length === 0}
    <div class="flex h-32 w-full flex-col items-center justify-center">
      <p class="text-muted-foreground mb-2 text-center">
        No tracks available. Wait a second, we are scanning your music library.
      </p>
    </div>
  {:else if virtualizer}
    <div class="relative" style="height: {$virtualizer!.getTotalSize()}px">
      {#each $virtualizer!.getVirtualItems() as item (tracks[item.index]?.id || `missing-${item.index}`)}
        {#if tracks[item.index]}
          <div
            style="position: absolute; width: 100%; transform: translateY({item.start}px)"
            data-index={item.index}
            use:measureElement
          >
            {#if $trackStore.currentTrackIndex === item.index}
              <TrackItem
                track={tracks[item.index]}
                index={item.index}
                isSelected={true}
                {audioService}
                {currentTime}
                {duration}
                {isPlaying}
                {bufferedRanges}
              />
            {:else}
              <TrackItem
                track={tracks[item.index]}
                index={item.index}
                isSelected={false}
                {audioService}
              />
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>
