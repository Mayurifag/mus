<script lang="ts">
  import type { Track, TimeRange } from "$lib/types";
  import type { AudioService } from "$lib/services/AudioService";
  import { trackStore } from "$lib/stores/trackStore";
  import { Play, Pause } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import { Slider } from "$lib/components/ui/slider";

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

  function formatDuration(seconds: number): string {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function playTrack() {
    if (!audioService) {
      // If no audioService, just select the track
      trackStore.playTrack(index);
      return;
    }

    if (isSelected) {
      // If this track is already selected, toggle play/pause
      if (localIsPlaying) {
        audioService.pause();
      } else {
        audioService.play();
      }
    } else {
      // Play a different track - this will trigger auto-play in layout
      trackStore.playTrack(index);
    }
  }

  function handlePlayButtonClick(event: MouseEvent) {
    event.stopPropagation();
    playTrack();
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      playTrack();
    }
  }

  let progressValue = $state([0]);
  let isUserDragging = $state(false);
  let localIsPlaying = $state(false);
  let duration = $state(0);
  let bufferedRanges = $state<TimeRange[]>([]);

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
            if (!isUserDragging) {
              // For AI assistant: Debug logging - do not remove this log, user intended to have it
              console.log(
                "TrackItem $effect: currentTime updated for track",
                track.id,
                "time:",
                time,
                "isUserDragging:",
                isUserDragging,
              );
              progressValue = [time];
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
      progressValue = [0];
      duration = 0;
      bufferedRanges = [];
    }
  });

  function handleProgressChange(value: number[]): void {
    if (audioService) {
      audioService.setCurrentTime(value[0]);
    }
  }

  function handleProgressCommit(): void {
    isUserDragging = false;
  }

  function handleProgressInput(event: Event): void {
    event.stopPropagation(); // Prevent track selection when dragging slider
    isUserDragging = true;
  }

  function handleSliderContainerClick(event: Event): void {
    event.stopPropagation();
  }
</script>

<div
  class="hover:bg-muted/50 flex cursor-pointer items-center gap-4 rounded-md p-2 transition-colors {isSelected
    ? 'bg-secondary'
    : ''}"
  onclick={playTrack}
  onkeydown={handleKeyDown}
  role="button"
  tabindex="0"
  data-testid="track-item"
  id="track-item-{track.id}"
>
  <div
    class="flex h-14 w-14 flex-shrink-0 items-center justify-center overflow-hidden rounded-md"
  >
    {#if track.has_cover && track.cover_small_url}
      <img
        src={track.cover_small_url}
        alt="Album art for {track.title}"
        class="h-full w-full object-cover"
        loading="lazy"
      />
    {:else}
      <div class="bg-muted flex h-full w-full items-center justify-center">
        <span class="text-muted-foreground text-xs">No Cover</span>
      </div>
    {/if}
  </div>

  <div class="flex min-w-0 flex-1 flex-col">
    <span class="truncate font-medium">{track.title}</span>
    <span class="text-muted-foreground truncate text-sm">{track.artist}</span>

    {#if isSelected}
      <Slider
        bind:value={progressValue}
        onValueChange={handleProgressChange}
        onValueCommit={handleProgressCommit}
        onInput={handleProgressInput}
        max={duration || 100}
        step={1}
        class="relative z-10 mt-1 w-full cursor-pointer"
        data-testid="track-progress-slider"
        onclick={handleSliderContainerClick}
        {bufferedRanges}
      />
    {/if}
  </div>

  <div class="text-muted-foreground flex flex-col items-end text-sm">
    <span>{formatDuration(track.duration)}</span>
  </div>

  <Button
    variant="ghost"
    size="icon"
    class="h-8 w-8"
    onclick={handlePlayButtonClick}
    aria-label={isSelected && localIsPlaying ? "Pause track" : "Play track"}
  >
    {#if isSelected && localIsPlaying}
      <Pause class="h-4 w-4" />
    {:else}
      <Play class="h-4 w-4" />
    {/if}
  </Button>
</div>
