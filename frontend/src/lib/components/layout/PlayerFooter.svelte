<script lang="ts">
  import { Card } from "$lib/components/ui/card";
  import { Button } from "$lib/components/ui/button";
  import { Slider } from "$lib/components/ui/slider";
  import { trackStore } from "$lib/stores/trackStore";
  import type { AudioService } from "$lib/services/AudioService";
  import type { TimeRange } from "$lib/types";
  import { formatArtistsForDisplay } from "$lib/utils";
  import {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Volume2,
    VolumeX,
    Menu,
    Shuffle,
    Repeat,
    Repeat1,
  } from "@lucide/svelte";
  import { browser } from "$app/environment";

  // Accept AudioService as a prop using Svelte 5 syntax
  let { audioService }: { audioService?: AudioService } = $props();

  function formatTime(seconds: number): string {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  let isVolumeHovered = $state(false);
  let progressValue = $state(0);
  let volumeValue = $state(1);
  let isDraggingProgress = $state(false);

  // Reactive state from AudioService stores
  let isPlaying = $state(false);
  let isMuted = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let isRepeat = $state(false);
  let bufferedRanges = $state<TimeRange[]>([]);

  // Reactive volume feedback value
  let volumeFeedbackValue = $derived(
    isMuted ? 0 : Math.round(volumeValue * 100),
  );

  // Sync local state with AudioService stores when they change
  $effect(() => {
    if (audioService?.isPlayingStore) {
      const unsubscribe = audioService.isPlayingStore.subscribe((playing) => {
        isPlaying = playing;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.isMutedStore) {
      const unsubscribe = audioService.isMutedStore.subscribe((muted) => {
        isMuted = muted;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.currentTimeStore) {
      const unsubscribe = audioService.currentTimeStore.subscribe((time) => {
        currentTime = time;
        if (!isDraggingProgress) {
          progressValue = time;
        }
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.durationStore) {
      const unsubscribe = audioService.durationStore.subscribe((dur) => {
        duration = dur;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.volumeStore) {
      const unsubscribe = audioService.volumeStore.subscribe((volume) => {
        volumeValue = volume;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.isRepeatStore) {
      const unsubscribe = audioService.isRepeatStore.subscribe((repeat) => {
        isRepeat = repeat;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.currentBufferedRangesStore) {
      const unsubscribe = audioService.currentBufferedRangesStore.subscribe(
        (ranges) => {
          bufferedRanges = ranges;
        },
      );
      return unsubscribe;
    }
  });

  function handleProgressCommit(): void {
    if (audioService) {
      audioService.setCurrentTime(progressValue);
    }
    isDraggingProgress = false;
  }

  function handleProgressInput(): void {
    isDraggingProgress = true;
  }

  function handleVolumeChange(value: number): void {
    if (audioService) {
      audioService.setVolume(value);
    }
  }

  function toggleMenu() {
    if (browser) {
      const event = new CustomEvent("toggle-sheet");
      document.body.dispatchEvent(event);
    }
  }

  function triggerManualScroll() {
    if (browser) {
      const event = new CustomEvent("force-scroll");
      document.body.dispatchEvent(event);
    }
  }
</script>

<div
  class="bg-card sm700:h-[var(--footer-height-desktop)] fixed bottom-0 left-0 right-0 z-50 h-[var(--footer-height-mobile)] border-t"
>
  <Card class="h-full rounded-none border-0 shadow-none">
    <!-- Mobile Layout (< 700px) -->
    <div
      class="sm700:hidden flex h-[var(--footer-height-mobile)] flex-col justify-between p-3"
    >
      <!-- Row 1: Controls -->
      <div class="flex w-full items-center justify-center gap-4">
        <!-- Shuffle Button -->
        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect h-12 w-12 {$trackStore.is_shuffle
            ? 'bg-accent/10'
            : ''}"
          onclick={() => trackStore.toggleShuffle()}
          aria-label="Toggle Shuffle"
          aria-pressed={$trackStore.is_shuffle}
        >
          <Shuffle
            class="h-6 w-6"
            color={$trackStore.is_shuffle
              ? "hsl(var(--accent))"
              : "currentColor"}
          />
        </Button>

        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect h-12 w-12"
          onclick={() => trackStore.previousTrack()}
          aria-label="Previous Track"
          disabled={!$trackStore.currentTrack}
        >
          <SkipBack class="h-6 w-6" />
        </Button>

        <!-- Play/Pause Button (centered) -->
        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect h-14 w-14"
          onclick={() => {
            if (audioService) {
              if (isPlaying) {
                audioService.pause();
              } else {
                audioService.play();
              }
            }
          }}
          aria-label={isPlaying ? "Pause" : "Play"}
          disabled={!$trackStore.currentTrack}
        >
          {#if isPlaying}
            <Pause class="h-7 w-7" />
          {:else}
            <Play class="h-7 w-7" />
          {/if}
        </Button>

        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect h-12 w-12"
          onclick={() => trackStore.nextTrack()}
          aria-label="Next Track"
          disabled={!$trackStore.currentTrack}
        >
          <SkipForward class="h-6 w-6" />
        </Button>

        <!-- Repeat Button -->
        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect h-12 w-12 {isRepeat ? 'bg-accent/10' : ''}"
          onclick={() => {
            if (audioService) {
              audioService.toggleRepeat();
            }
          }}
          aria-label="Toggle Repeat"
          aria-pressed={isRepeat}
        >
          {#if isRepeat}
            <Repeat1 class="h-6 w-6" color="hsl(var(--accent))" />
          {:else}
            <Repeat class="h-6 w-6" color="currentColor" />
          {/if}
        </Button>
      </div>

      <!-- Row 2: Volume Controls (centered, no menu) -->
      <div class="flex w-full items-center justify-center gap-3">
        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect h-12 w-12 flex-shrink-0"
          onclick={() => {
            if (audioService) {
              audioService.toggleMute();
            }
          }}
          aria-label={isMuted ? "Unmute" : "Mute"}
        >
          {#if isMuted}
            <VolumeX class="h-6 w-6" />
          {:else}
            <Volume2 class="h-6 w-6" />
          {/if}
        </Button>
        <div
          class="relative w-32 flex-shrink-0 cursor-pointer"
          role="slider"
          aria-label="Volume control"
          aria-valuenow={volumeFeedbackValue}
          aria-valuemin="0"
          aria-valuemax="100"
          tabindex="0"
          onmouseenter={() => (isVolumeHovered = true)}
          onmouseleave={() => (isVolumeHovered = false)}
        >
          <Slider
            bind:value={volumeValue}
            onValueChange={handleVolumeChange}
            max={1}
            step={0.01}
            class="w-full"
          />
          {#if isVolumeHovered}
            <div
              class="bg-muted absolute -top-7 left-1/2 -translate-x-1/2 rounded px-2 py-1 text-xs font-medium text-white transition-opacity"
            >
              {volumeFeedbackValue}%
            </div>
          {/if}
        </div>
      </div>

      <!-- Row 3: Progress -->
      <div class="flex w-full items-center gap-3 px-4">
        <span class="text-muted-foreground w-10 text-right text-xs">
          {formatTime(currentTime)}
        </span>
        <Slider
          bind:value={progressValue}
          onValueCommit={handleProgressCommit}
          onInput={handleProgressInput}
          max={duration || 100}
          step={1}
          class="flex-1 cursor-pointer"
          disabled={!$trackStore.currentTrack}
          {bufferedRanges}
        />
        <span class="text-muted-foreground w-10 text-xs">
          {formatTime(duration)}
        </span>
      </div>

      <!-- Row 4: Track Info -->
      <div class="flex w-full items-center justify-center px-4 text-center">
        {#if $trackStore.currentTrack}
          <span class="text-muted-foreground truncate text-sm font-medium">
            {formatArtistsForDisplay($trackStore.currentTrack.artist)} - {$trackStore
              .currentTrack.title}
          </span>
        {:else}
          <span class="text-muted-foreground text-sm">Not Playing</span>
        {/if}
      </div>
    </div>

    <!-- Desktop Layout (>= 700px) -->
    <div
      class="sm700:flex hidden h-[var(--footer-height-desktop)] items-center pr-4"
    >
      <!-- Track Info -->
      <div class="desktop:w-80 flex w-auto min-w-0 items-center">
        {#if $trackStore.currentTrack}
          <div
            class="desktop:block desktop:h-24 desktop:w-24 desktop:my-5 desktop:ml-5 my-6 ml-6 hidden h-32 w-32 flex-shrink-0 cursor-pointer overflow-hidden rounded-md"
            onclick={triggerManualScroll}
            onkeydown={(e) => e.key === "Enter" && triggerManualScroll()}
            role="button"
            tabindex="0"
            aria-label="Scroll to current track"
          >
            {#if $trackStore.currentTrack.has_cover && $trackStore.currentTrack.cover_original_url}
              <img
                src={$trackStore.currentTrack.cover_original_url}
                alt="Album Cover"
                class="h-full w-full object-cover"
              />
            {:else}
              <img
                src="/images/no-cover.svg"
                alt="No Album Cover"
                class="h-full w-full object-cover"
              />
            {/if}
          </div>
          <div class="sm700:flex ml-4 hidden min-w-0 flex-col overflow-hidden">
            <span
              class="cursor-pointer truncate text-base font-medium hover:underline"
              onclick={triggerManualScroll}
              onkeydown={(e) => e.key === "Enter" && triggerManualScroll()}
              role="button"
              tabindex="0"
              aria-label="Scroll to current track"
              >{$trackStore.currentTrack.title}</span
            >
            <span class="text-muted-foreground truncate text-sm">
              {formatArtistsForDisplay($trackStore.currentTrack.artist)}
            </span>
          </div>
        {:else}
          <div
            class="desktop:block desktop:h-24 desktop:w-24 desktop:my-5 desktop:ml-5 bg-muted my-6 ml-6 flex hidden h-32 w-32 flex-shrink-0 items-center justify-center rounded-md"
          >
            <span class="text-muted-foreground text-xs">No Track</span>
          </div>
          <div class="sm700:flex ml-4 hidden min-w-0 flex-col overflow-hidden">
            <span class="text-muted-foreground text-sm">Not Playing</span>
          </div>
        {/if}
      </div>

      <!-- Controls -->
      <div
        class="desktop:justify-center desktop:py-0 desktop:mx-0 sm700:mx-2 mx-4 flex h-full flex-1 flex-col items-center justify-around py-1"
      >
        <!-- Desktop Row 1: Control Buttons & Volume Controls -->
        <div
          class="desktop:space-x-2 flex w-full items-center justify-center space-x-2"
        >
          <!-- Shuffle Button -->
          <Button
            variant="ghost"
            size="icon"
            class="icon-glow-effect h-9 w-9 {$trackStore.is_shuffle
              ? 'bg-accent/10'
              : ''}"
            onclick={() => trackStore.toggleShuffle()}
            aria-label="Toggle Shuffle"
            aria-pressed={$trackStore.is_shuffle}
          >
            <Shuffle
              class="h-5 w-5"
              color={$trackStore.is_shuffle
                ? "hsl(var(--accent))"
                : "currentColor"}
            />
          </Button>

          <!-- Repeat Button -->
          <Button
            variant="ghost"
            size="icon"
            class="icon-glow-effect h-9 w-9 {isRepeat ? 'bg-accent/10' : ''}"
            onclick={() => {
              if (audioService) {
                audioService.toggleRepeat();
              }
            }}
            aria-label="Toggle Repeat"
            aria-pressed={isRepeat}
          >
            {#if isRepeat}
              <Repeat1 class="h-5 w-5" color="hsl(var(--accent))" />
            {:else}
              <Repeat class="h-5 w-5" color="currentColor" />
            {/if}
          </Button>

          <Button
            variant="ghost"
            size="icon"
            class="icon-glow-effect h-10 w-10"
            onclick={() => trackStore.previousTrack()}
            aria-label="Previous Track"
            disabled={!$trackStore.currentTrack}
          >
            <SkipBack class="h-6 w-6" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="icon-glow-effect h-12 w-12"
            onclick={() => {
              if (audioService) {
                if (isPlaying) {
                  audioService.pause();
                } else {
                  audioService.play();
                }
              }
            }}
            aria-label={isPlaying ? "Pause" : "Play"}
            disabled={!$trackStore.currentTrack}
          >
            {#if isPlaying}
              <Pause class="h-7 w-7" />
            {:else}
              <Play class="h-7 w-7" />
            {/if}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="icon-glow-effect h-10 w-10"
            onclick={() => trackStore.nextTrack()}
            aria-label="Next Track"
            disabled={!$trackStore.currentTrack}
          >
            <SkipForward class="h-6 w-6" />
          </Button>

          <!-- Volume Controls -->
          <Button
            variant="ghost"
            size="icon"
            class="icon-glow-effect h-9 w-9"
            onclick={() => {
              if (audioService) {
                audioService.toggleMute();
              }
            }}
            aria-label={isMuted ? "Unmute" : "Mute"}
          >
            {#if isMuted}
              <VolumeX class="h-5 w-5" />
            {:else}
              <Volume2 class="h-5 w-5" />
            {/if}
          </Button>
          <div
            class="relative w-24 cursor-pointer"
            role="slider"
            aria-label="Volume control"
            aria-valuenow={volumeFeedbackValue}
            aria-valuemin="0"
            aria-valuemax="100"
            tabindex="0"
            onmouseenter={() => (isVolumeHovered = true)}
            onmouseleave={() => (isVolumeHovered = false)}
          >
            <Slider
              bind:value={volumeValue}
              onValueChange={handleVolumeChange}
              max={1}
              step={0.01}
              class="w-full"
            />
            {#if isVolumeHovered}
              <div
                class="bg-muted absolute -top-7 left-1/2 -translate-x-1/2 rounded px-2 py-1 text-xs font-medium text-white transition-opacity"
              >
                {volumeFeedbackValue}%
              </div>
            {/if}
          </div>
        </div>

        <!-- Desktop Row 2: Progress Slider & Time Indicators -->
        <div
          class="desktop:mt-2 desktop:max-w-lg mb-2 flex w-full max-w-md items-center space-x-2"
        >
          <span class="text-muted-foreground w-10 text-right text-xs">
            {formatTime(currentTime)}
          </span>
          <Slider
            bind:value={progressValue}
            onValueCommit={handleProgressCommit}
            onInput={handleProgressInput}
            max={duration || 100}
            step={1}
            class="flex-1 cursor-pointer"
            disabled={!$trackStore.currentTrack}
            {bufferedRanges}
          />
          <span class="text-muted-foreground w-10 text-xs">
            {formatTime(duration)}
          </span>
        </div>
      </div>

      <!-- Additional Controls -->
      <div
        class="flex w-auto flex-shrink-0 items-center justify-end space-x-2 pr-4"
      >
        <!-- Desktop Menu Button (hidden on mobile) -->
        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect desktop:hidden ml-2"
          onclick={toggleMenu}
          aria-label="Open menu"
        >
          <Menu class="h-5 w-5" />
        </Button>
      </div>
    </div>
  </Card>
</div>

<style>
  :global(.icon-glow-effect svg) {
    color: white;
    transition: all 0.1s ease;
  }

  :global(.icon-glow-effect:hover svg) {
    color: white;
    filter: drop-shadow(0 0 8px hsl(var(--accent) / 0.8))
      drop-shadow(0 0 4px hsl(var(--accent) / 1));
  }
</style>
