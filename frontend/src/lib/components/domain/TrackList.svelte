<script lang="ts">
  import { tick } from "svelte";
  import type { AudioService } from "$lib/services/AudioService";
  import type { TimeRange, Track } from "$lib/types";
  import TrackItem from "./TrackItem.svelte";
  import LazyTrackMetadataModal from "./LazyTrackMetadataModal.svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { browser } from "$app/env";
  import { updateEffectStats } from "$lib/utils/monitoredEffect";
  import { parseArtists } from "$lib/utils/formatters";
  import { ArrowLeft } from "@lucide/svelte";
  import { clearArtistFilter } from "$lib/utils/artistFilterNavigation";
  import {
    categoryLabels,
    trackCategoryNames,
  } from "$lib/utils/categoryMarkers";

  let { audioService }: { audioService?: AudioService } = $props();

  const selectedArtist = $derived($trackStore.selectedArtist);
  const selectedCategory = $derived($trackStore.selectedCategory);
  const selectedCategoryLabel = $derived(
    selectedCategory
      ? (categoryLabels[selectedCategory] ?? selectedCategory)
      : "",
  );
  const trackRows = $derived.by(() =>
    $trackStore.tracks.reduce<{ track: Track; index: number }[]>(
      (rows, track, index) => {
        if (
          (!selectedArtist ||
            parseArtists(track.artist).includes(selectedArtist)) &&
          (!selectedCategory ||
            trackCategoryNames(track).includes(selectedCategory))
        ) {
          rows.push({ track, index });
        }
        return rows;
      },
      [],
    ),
  );
  const visibleTrackCount = $derived(trackRows.length);

  // Audio playback state variables
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let bufferedRanges = $state<TimeRange[]>([]);

  let editModalOpen = $state(false);
  let editingTrack = $state<Track | null>(null);
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
    if (!browser || initialScrollDone || $trackStore.tracksLoading) return;

    initialScrollDone = true;
    const trackId = $trackStore.currentTrack?.id;
    if (trackId && trackRows.some((row) => row.track.id === trackId)) {
      void tick().then(() => {
        document.getElementById(`track-item-${trackId}`)?.scrollIntoView({
          block: "center",
        });
      });
    }
  });

  function clearActiveFilter() {
    if (selectedArtist) {
      clearArtistFilter();
    } else {
      trackStore.clearCategoryFilter();
    }
  }
</script>

<div data-testid="track-list" aria-busy={$trackStore.tracksLoading}>
  {#if selectedArtist || selectedCategory}
    <button
      type="button"
      class="border-border/40 bg-card/60 hover:bg-muted/30 mx-3 mb-3 flex w-[calc(100%-1.5rem)] cursor-pointer items-center gap-3 rounded-xl border px-3 py-3 text-left shadow-sm transition-colors"
      onclick={clearActiveFilter}
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
          {selectedArtist ?? selectedCategoryLabel}
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

  {#if $trackStore.tracksLoading}
    <div role="status" aria-label="Loading tracks">
      <span class="sr-only">Loading tracks...</span>
      <div class="motion-safe:animate-pulse" aria-hidden="true">
        {#each [60, 45, 70, 50, 65, 40, 55, 68] as width (width)}
          <div class="flex items-center gap-4 p-2">
            <div class="bg-muted/30 h-14 w-14 shrink-0 rounded-md"></div>
            <div class="flex min-w-0 flex-1 flex-col gap-2">
              <div
                class="bg-muted/30 h-4 max-w-72 rounded"
                style="width: {width}%"
              ></div>
              <div class="bg-muted/20 h-3 w-28 max-w-full rounded"></div>
            </div>
            <div class="bg-muted/20 h-3 w-8 rounded"></div>
          </div>
        {/each}
      </div>
    </div>
  {:else if $trackStore.tracks.length === 0}
    <div class="flex h-32 w-full flex-col items-center justify-center">
      <p class="text-muted-foreground mb-2 text-center">
        No tracks available. Wait a second, we are scanning your music library.
      </p>
    </div>
  {:else if visibleTrackCount === 0}
    <div class="flex h-32 w-full flex-col items-center justify-center">
      <p class="text-muted-foreground mb-2 text-center">
        No songs found for this filter.
      </p>
    </div>
  {:else}
    {#each trackRows as row (row.track.id)}
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
          {audioService}
          onEdit={openEditModal}
        />
      {/if}
    {/each}
  {/if}
</div>

{#if editingTrack}
  <LazyTrackMetadataModal
    bind:open={editModalOpen}
    mode="edit"
    track={editingTrack}
    onClose={closeEditModal}
  />
{/if}
