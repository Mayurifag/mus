<script lang="ts">
  import { tick } from "svelte";
  import type { AudioService } from "$lib/services/AudioService";
  import type { TimeRange, Track } from "$lib/types";
  import TrackItem from "./TrackItem.svelte";
  import TrackMetadataModal from "./TrackMetadataModal.svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { createWindowVirtualizer } from "@tanstack/svelte-virtual";
  import { browser } from "$app/environment";
  import { updateEffectStats } from "$lib/utils/monitoredEffect";
  import { parseArtists } from "$lib/utils/formatters";
  import { get } from "svelte/store";
  import { ArrowLeft } from "@lucide/svelte";
  import { clearArtistFilter } from "$lib/utils/artistFilterNavigation";

  let { audioService }: { audioService?: AudioService } = $props();

  const selectedArtist = $derived($trackStore.selectedArtist);
  const trackRows = $derived.by(() =>
    $trackStore.tracks.reduce<{ track: Track; index: number }[]>(
      (rows, track, index) => {
        if (
          !selectedArtist ||
          parseArtists(track.artist).includes(selectedArtist)
        ) {
          rows.push({ track, index });
        }
        return rows;
      },
      [],
    ),
  );
  const visibleTrackCount = $derived(trackRows.length);
  const shouldVirtualize = $derived(visibleTrackCount >= 500);
  const currentVisibleIndex = $derived(
    trackRows.findIndex((row) => row.track.id === $trackStore.currentTrack?.id),
  );

  // Audio playback state variables
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let bufferedRanges = $state<TimeRange[]>([]);

  const virtualizer = browser
    ? createWindowVirtualizer(virtualizerOptions(0))
    : null;
  let virtualizerCount = -1;
  let editModalOpen = $state(false);
  let editingTrack = $state<Track | null>(null);
  let initialScrollDone = false;

  function virtualizerOptions(count: number) {
    return {
      count,
      estimateSize: () => 72,
      overscan: 12,
      getScrollElement: () => window,
    };
  }

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
    updateEffectStats("TrackList_VirtualizerCountUpdate");

    if (
      virtualizer &&
      shouldVirtualize &&
      virtualizerCount !== visibleTrackCount
    ) {
      virtualizerCount = visibleTrackCount;
      $virtualizer!.setOptions(virtualizerOptions(visibleTrackCount));
    }
  });

  function openEditModal(track: Track) {
    editingTrack = track;
    editModalOpen = true;
  }

  function closeEditModal() {
    editModalOpen = false;
    editingTrack = null;
  }

  $effect(() => {
    if (!editModalOpen && editingTrack) {
      editingTrack = null;
    }
  });

  $effect(() => {
    updateEffectStats("TrackList_InitialScroll");

    if (
      !initialScrollDone &&
      virtualizer &&
      shouldVirtualize &&
      currentVisibleIndex !== -1
    ) {
      const currentVirtualizer = virtualizer;
      tick().then(() => {
        if (currentVisibleIndex !== -1) {
          get(currentVirtualizer).scrollToIndex(currentVisibleIndex, {
            align: "center",
          });
        }
      });
      initialScrollDone = true;
    }
  });

  function measureElement(node: HTMLElement) {
    if (virtualizer && shouldVirtualize) {
      get(virtualizer).measureElement(node);
    }
  }
</script>

{#snippet renderTrack(row: { track: Track; index: number })}
  {#if $trackStore.currentTrack?.id === row.track.id}
    <TrackItem
      track={row.track}
      index={row.index}
      isSelected={true}
      {audioService}
      {currentTime}
      {duration}
      {isPlaying}
      {bufferedRanges}
      onEdit={openEditModal}
    />
  {:else}
    <TrackItem
      track={row.track}
      index={row.index}
      isSelected={false}
      {audioService}
      onEdit={openEditModal}
    />
  {/if}
{/snippet}

<div class="flex flex-col" data-testid="track-list">
  {#if selectedArtist}
    <button
      type="button"
      class="border-border/40 bg-card/60 hover:bg-muted/30 mx-3 mb-3 flex w-[calc(100%-1.5rem)] cursor-pointer items-center gap-3 rounded-xl border px-3 py-3 text-left shadow-sm transition-colors"
      onclick={clearArtistFilter}
      aria-label="Back to all songs"
    >
      <span
        class="bg-muted/50 text-muted-foreground flex h-10 w-10 shrink-0 items-center justify-center rounded-full"
      >
        <ArrowLeft class="h-5 w-5" />
      </span>
      <span class="min-w-0 flex-1">
        <span class="text-muted-foreground block text-xs font-medium">
          Back to all songs
        </span>
        <span class="block truncate text-base font-semibold">
          {selectedArtist}
        </span>
      </span>
      <span
        class="border-border/50 text-muted-foreground shrink-0 rounded-full border px-2.5 py-1 text-xs"
      >
        {visibleTrackCount}
        {visibleTrackCount === 1 ? "song" : "songs"}
      </span>
    </button>
  {/if}

  {#if $trackStore.tracks.length === 0}
    <div class="flex h-32 w-full flex-col items-center justify-center">
      <p class="text-muted-foreground mb-2 text-center">
        No tracks available. Wait a second, we are scanning your music library.
      </p>
    </div>
  {:else if visibleTrackCount === 0}
    <div class="flex h-32 w-full flex-col items-center justify-center">
      <p class="text-muted-foreground mb-2 text-center">
        No songs found for this artist.
      </p>
    </div>
  {:else if shouldVirtualize && virtualizer}
    <div class="relative" style="height: {$virtualizer!.getTotalSize()}px">
      {#each $virtualizer!.getVirtualItems() as item (trackRows[item.index]?.track.id || `missing-${item.index}`)}
        {#if trackRows[item.index]}
          <div
            style="position: absolute; width: 100%; transform: translateY({item.start}px)"
            data-index={item.index}
            use:measureElement
          >
            {@render renderTrack(trackRows[item.index])}
          </div>
        {/if}
      {/each}
    </div>
  {:else}
    {#each trackRows as row (row.track.id)}
      {@render renderTrack(row)}
    {/each}
  {/if}
</div>

{#if editingTrack}
  <TrackMetadataModal
    bind:open={editModalOpen}
    mode="edit"
    track={editingTrack}
    onClose={closeEditModal}
  />
{/if}
