<script lang="ts">
  import type { AudioService } from "$lib/services/AudioService";
  import TrackItem from "./TrackItem.svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { createWindowVirtualizer } from "@tanstack/svelte-virtual";
  import { browser } from "$app/environment";

  let { audioService }: { audioService?: AudioService } = $props();

  const tracks = $derived($trackStore.tracks);

  let virtualizer = $state<ReturnType<typeof createWindowVirtualizer> | null>(
    null,
  );
  let initialScrollDone = $state(false);

  $effect(() => {
    if (browser) {
      virtualizer = createWindowVirtualizer({
        count: tracks.length,
        estimateSize: () => 72,
        overscan: 15,
        getScrollElement: () => window,
      });
    }
  });

  $effect(() => {
    if (
      !initialScrollDone &&
      virtualizer &&
      $trackStore.currentTrackIndex !== null
    ) {
      setTimeout(() => {
        if (virtualizer && $trackStore.currentTrackIndex !== null) {
          $virtualizer!.scrollToIndex($trackStore.currentTrackIndex, {
            align: "center",
          });
        }
      }, 50);
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
      {#each $virtualizer!.getVirtualItems() as item (tracks[item.index].id)}
        <div
          style="position: absolute; width: 100%; transform: translateY({item.start}px)"
          data-index={item.index}
          use:measureElement
        >
          <TrackItem
            track={tracks[item.index]}
            index={item.index}
            isSelected={$trackStore.currentTrackIndex === item.index}
            {audioService}
          />
        </div>
      {/each}
    </div>
  {/if}
</div>
