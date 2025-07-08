<script lang="ts">
  import type { Track, TimeRange } from "$lib/types";
  import type { AudioService } from "$lib/services/AudioService";
  import { trackStore } from "$lib/stores/trackStore";
  import { permissionsStore } from "$lib/stores/permissionsStore";

  import { Slider } from "$lib/components/ui/slider";
  import { Pencil } from "@lucide/svelte";
  import EditTrackModal from "./EditTrackModal.svelte";
  import { formatArtistsForDisplay } from "$lib/utils";

  let {
    track,
    isSelected = false,
    index,
    audioService,
  }: {
    track: Track;
    isSelected?: boolean;
    index: number;
    audioService?: AudioService;
  } = $props();

  // Make track data reactive to prop changes
  const trackTitle = $derived(track.title);
  const trackArtist = $derived(track.artist);
  const trackDuration = $derived(track.duration);
  const trackHasCover = $derived(track.has_cover);
  const trackCoverSmallUrl = $derived(track.cover_small_url);

  function formatDuration(seconds: number): string {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function playTrack() {
    if (!audioService) {
      trackStore.playTrack(index);
      return;
    }

    if (isSelected) {
      if (localIsPlaying) {
        audioService.pause();
      } else {
        audioService.play();
      }
    } else {
      trackStore.playTrack(index);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      playTrack();
    }
  }

  let progressValue = $state(0);
  let localIsPlaying = $state(false);
  let duration = $state(0);
  let bufferedRanges = $state<TimeRange[]>([]);
  let editModalOpen = $state(false);

  let mouseDownTarget: EventTarget | null = null;
  let isDragging = $state(false);

  function handleMouseDown(event: MouseEvent): void {
    if (event.button !== 0) return;
    mouseDownTarget = event.target;
  }

  function handleMouseUp(event: MouseEvent): void {
    if (event.button !== 0) return;

    const isValidClick = mouseDownTarget === event.target;

    mouseDownTarget = null;

    if (isValidClick) {
      playTrack();
    }
  }

  $effect(() => {
    if (isSelected && audioService) {
      const unsubscribers: (() => void)[] = [];

      // eslint-disable-next-line svelte/require-store-reactive-access
      if (audioService.isPlayingStore) {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log(
          "TrackItem $effect: isPlayingStore subscription for track",
          track.id,
          track.title,
        );
        unsubscribers.push(
          audioService.isPlayingStore.subscribe((playing) => {
            // For AI assistant: Debug logging - do not remove this log, user intended to have it
            console.log(
              "TrackItem $effect: isPlaying changed for track",
              track.id,
              "Playing:",
              playing,
            );
            localIsPlaying = playing;
          }),
        );
      }

      // eslint-disable-next-line svelte/require-store-reactive-access
      if (audioService.currentTimeStore) {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log(
          "TrackItem $effect: currentTimeStore subscription for track",
          track.id,
          "isSelected:",
          isSelected,
        );
        unsubscribers.push(
          audioService.currentTimeStore.subscribe((time) => {
            if (!isDragging) {
              // For AI assistant: Debug logging - do not remove this log, user intended to have it
              console.log(
                "TrackItem $effect: currentTime updated for track",
                track.id,
                "time:",
                time,
              );
              progressValue = time;
            }
          }),
        );
      }

      // eslint-disable-next-line svelte/require-store-reactive-access
      if (audioService.durationStore) {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log(
          "TrackItem $effect: durationStore subscription for track",
          track.id,
        );
        unsubscribers.push(
          audioService.durationStore.subscribe((dur) => {
            // For AI assistant: Debug logging - do not remove this log, user intended to have it
            console.log(
              "TrackItem $effect: duration updated for track",
              track.id,
              "duration:",
              dur,
            );
            duration = dur;
          }),
        );
      }

      // eslint-disable-next-line svelte/require-store-reactive-access
      if (audioService.currentBufferedRangesStore) {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log(
          "TrackItem $effect: currentBufferedRangesStore subscription for track",
          track.id,
        );
        unsubscribers.push(
          audioService.currentBufferedRangesStore.subscribe((ranges) => {
            // For AI assistant: Debug logging - do not remove this log, user intended to have it
            console.log(
              "TrackItem $effect: bufferedRanges updated for track",
              track.id,
              "ranges:",
              ranges,
            );
            bufferedRanges = ranges;
          }),
        );
      }

      return () => unsubscribers.forEach((unsub) => unsub());
    } else {
      localIsPlaying = false;
      progressValue = 0;
      duration = 0;
      bufferedRanges = [];
    }
  });

  function handleProgressCommit(): void {
    if (audioService) {
      audioService.setCurrentTime(progressValue);
    }
    isDragging = false;
  }

  function handleProgressInput(event: Event): void {
    event.stopPropagation();
    isDragging = true;
  }
</script>

<div
  class="hover:bg-muted/50 flex cursor-pointer items-center gap-4 rounded-md p-2 transition-colors {isSelected
    ? 'bg-secondary'
    : ''}"
  onmousedown={handleMouseDown}
  onmouseup={handleMouseUp}
  onkeydown={handleKeyDown}
  role="button"
  tabindex="0"
  data-testid="track-item"
  id="track-item-{track.id}"
>
  <div
    class="flex h-14 w-14 flex-shrink-0 items-center justify-center overflow-hidden rounded-md"
  >
    {#if trackHasCover && trackCoverSmallUrl}
      <img
        src={trackCoverSmallUrl}
        alt="Album art for {trackTitle}"
        class="h-full w-full object-cover"
        loading="lazy"
      />
    {:else}
      <img
        src="/images/no-cover.svg"
        alt="No Album Cover"
        class="h-full w-full object-cover"
        loading="lazy"
      />
    {/if}
  </div>

  <div class="flex min-w-0 flex-1 flex-col">
    <span class="truncate font-medium">{trackTitle}</span>
    <span class="text-muted-foreground truncate text-sm"
      >{formatArtistsForDisplay(trackArtist)}</span
    >

    {#if isSelected}
      <Slider
        bind:value={progressValue}
        onValueCommit={handleProgressCommit}
        onInput={handleProgressInput}
        max={duration || 100}
        step={1}
        class="relative z-10 mt-1 w-full cursor-pointer"
        data-testid="track-progress-slider"
        {bufferedRanges}
      />
    {/if}
  </div>

  <div class="text-muted-foreground flex flex-col items-end text-sm">
    <span>{formatDuration(trackDuration)}</span>
  </div>

  <div
    class="hidden md:block"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    onmousedown={(e) => e.stopPropagation()}
    onmouseup={(e) => e.stopPropagation()}
    role="button"
    tabindex="-1"
  >
    <button
      class="text-muted-foreground icon-glow-effect cursor-pointer rounded-md p-1 transition-colors"
      disabled={!$permissionsStore.can_write_files}
      onclick={() => (editModalOpen = true)}
      aria-label="Edit track"
      title={!$permissionsStore.can_write_files
        ? "Editing disabled: write permissions not available"
        : "Edit"}
    >
      <Pencil class="h-5 w-5" />
    </button>
  </div>
</div>

<EditTrackModal bind:open={editModalOpen} {track} />
