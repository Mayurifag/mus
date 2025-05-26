<script lang="ts">
  import type { Track } from "$lib/types";
  import { trackStore } from "$lib/stores/trackStore";
  import { playerStore } from "$lib/stores/playerStore";
  import { formatDistanceToNow } from "date-fns";
  import { Play, Pause } from "lucide-svelte";
  import { Button } from "$lib/components/ui/button";

  export let track: Track;
  export let isSelected = false;
  export let index: number;

  function formatDuration(seconds: number): string {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function getTimeSinceAdded(timestamp: number): string {
    return formatDistanceToNow(new Date(timestamp * 1000), { addSuffix: true });
  }

  function playTrack() {
    if (isSelected && $playerStore.isPlaying) {
      playerStore.pause();
    } else {
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

  function calculateProgress() {
    if (!isCurrentlyPlaying || $playerStore.duration <= 0) return 0;
    return ($playerStore.currentTime / $playerStore.duration) * 100;
  }

  $: isCurrentlyPlaying = isSelected && $playerStore.isPlaying;
  $: progressPercentage = isCurrentlyPlaying ? calculateProgress() : 0;
</script>

<div
  class="hover:bg-muted/50 flex cursor-pointer items-center gap-4 rounded-md p-2 transition-colors {isSelected
    ? 'bg-muted'
    : ''}"
  on:click={playTrack}
  on:keydown={handleKeyDown}
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
      />
    {:else}
      <div class="bg-muted flex h-full w-full items-center justify-center">
        <span class="text-muted-foreground text-xs">No Cover</span>
      </div>
    {/if}
  </div>

  <div class="flex flex-1 flex-col overflow-hidden">
    <span class="truncate font-medium">{track.title}</span>
    <span class="text-muted-foreground truncate text-sm">{track.artist}</span>

    {#if isSelected && $playerStore.isPlaying}
      <div class="bg-muted/50 mt-1 h-1 w-full overflow-hidden rounded-full">
        <div
          class="bg-primary h-full"
          style="width: {progressPercentage}%;"
          role="progressbar"
          aria-valuenow={progressPercentage}
          aria-valuemin="0"
          aria-valuemax="100"
          data-testid="track-progress-bar"
        ></div>
      </div>
    {/if}
  </div>

  <div class="text-muted-foreground flex flex-col items-end text-sm">
    <span>{formatDuration(track.duration)}</span>
    <span class="text-xs">{getTimeSinceAdded(track.added_at)}</span>
  </div>

  <Button
    variant="ghost"
    size="icon"
    class="h-8 w-8"
    on:click={handlePlayButtonClick}
    aria-label={isCurrentlyPlaying ? "Pause track" : "Play track"}
  >
    {#if isCurrentlyPlaying}
      <Pause class="h-4 w-4" />
    {:else}
      <Play class="h-4 w-4" />
    {/if}
  </Button>
</div>
