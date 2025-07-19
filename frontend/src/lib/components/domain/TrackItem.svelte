<script lang="ts">
  import type { Track, TimeRange } from "$lib/types";
  import type { AudioService } from "$lib/services/AudioService";
  import { trackStore } from "$lib/stores/trackStore";
  import { permissionsStore } from "$lib/stores/permissionsStore";
  import { Slider } from "$lib/components/ui/slider";
  import { Pencil } from "@lucide/svelte";
  import TrackMetadataModal from "./TrackMetadataModal.svelte";
  import { formatArtistsForDisplay, formatDuration } from "$lib/utils";

  let {
    track,
    isSelected = false,
    index,
    audioService,
    currentTime,
    duration,
    isPlaying,
    bufferedRanges,
  }: {
    track: Track;
    isSelected?: boolean;
    index: number;
    audioService?: AudioService;
    currentTime?: number;
    duration?: number;
    isPlaying?: boolean;
    bufferedRanges?: TimeRange[];
  } = $props();

  // Make track data reactive to prop changes
  const trackTitle = $derived(track.title);
  const trackArtist = $derived(track.artist);
  const trackDuration = $derived(track.duration);
  const trackHasCover = $derived(track.has_cover);
  const trackCoverSmallUrl = $derived(track.cover_small_url);

  function playTrack() {
    if (!audioService) {
      trackStore.playTrack(index);
      return;
    }

    if (isSelected) {
      if (isPlaying) {
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

  let progressValue = $derived(
    isSelected && currentTime !== undefined ? currentTime : 0,
  );
  let editModalOpen = $state(false);

  let mouseDownTarget: EventTarget | null = null;
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

  function handleProgressCommit(): void {
    if (audioService) {
      audioService.endSeeking(progressValue);
    }
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
        onpointerdown={() => audioService?.startSeeking()}
        onValueChange={(v: number[]) => audioService?.seek(v[0])}
        max={duration || 100}
        step={1}
        class="relative z-10 mt-1 w-full cursor-pointer"
        data-testid="track-progress-slider"
        bufferedRanges={bufferedRanges || []}
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

<TrackMetadataModal bind:open={editModalOpen} mode="edit" {track} />
