<script lang="ts">
  import { Card } from "$lib/components/ui/card";
  import { Button } from "$lib/components/ui/button";
  import { Slider } from "$lib/components/ui/slider";
  import { trackStore } from "$lib/stores/trackStore";
  import type { AudioService } from "$lib/services/AudioService";
  import type { TimeRange } from "$lib/types";
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

  // Volume feedback variables
  let showVolumeFeedback = $state(false);
  let volumeFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  // Local slider state for two-way binding - use AudioService if available
  let progressValue = $state([0]);
  let volumeValue = $state([1]);

  // Track if user is currently interacting with sliders
  let isUserDraggingProgress = $state(false);
  let isUserDraggingVolume = $state(false);

  // Reactive state from AudioService stores
  let isPlaying = $state(false);
  let isMuted = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let isRepeat = $state(false);
  let bufferedRanges = $state<TimeRange[]>([]);

  // Reactive volume feedback value
  let volumeFeedbackValue = $derived(
    isMuted ? 0 : Math.round((volumeValue[0] || 1) * 100),
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
        if (!isUserDraggingProgress) {
          progressValue = [time];
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
        if (!isUserDraggingVolume) {
          volumeValue = [volume];
        }
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

  function handleProgressChange(value: number[]): void {
    if (audioService) {
      audioService.setCurrentTime(value[0]);
    }
  }

  function handleProgressCommit(): void {
    isUserDraggingProgress = false;
  }

  function handleProgressInput(): void {
    isUserDraggingProgress = true;
  }

  function handleVolumeChange(value: number[]): void {
    if (audioService) {
      audioService.setVolume(value[0]);
    }

    // Show volume feedback
    showVolumeFeedback = true;

    // Clear existing timer if any
    if (volumeFeedbackTimer) {
      clearTimeout(volumeFeedbackTimer);
    }

    // Hide feedback after 1.5 seconds
    volumeFeedbackTimer = setTimeout(() => {
      showVolumeFeedback = false;
    }, 150);
  }

  function handleVolumeCommit(): void {
    isUserDraggingVolume = false;
  }

  function handleVolumeInput(): void {
    isUserDraggingVolume = true;
  }

  // Create a function to dispatch a custom event to toggle the sidebar
  function toggleMenu() {
    if (browser) {
      const event = new CustomEvent("toggle-sheet");
      document.body.dispatchEvent(event);
    }
  }

  // Clean up timer when component is destroyed using $effect
  $effect(() => {
    return () => {
      if (volumeFeedbackTimer) {
        clearTimeout(volumeFeedbackTimer);
      }
    };
  });
</script>

<div class="bg-card fixed right-0 bottom-0 left-0 z-50 border-t">
  <Card class="rounded-none border-0 shadow-none">
    <div class="flex h-28 items-center px-4">
      <!-- Track Info -->
      <div class="flex w-80 min-w-0 items-center space-x-4">
        {#if $trackStore.currentTrack}
          <div class="h-18 w-18 flex-shrink-0 overflow-hidden rounded-md">
            {#if $trackStore.currentTrack.has_cover && $trackStore.currentTrack.cover_original_url}
              <img
                src={$trackStore.currentTrack.cover_original_url}
                alt="Album Cover"
                class="h-full w-full object-cover"
              />
            {:else}
              <div
                class="bg-muted flex h-full w-full items-center justify-center"
              >
                <span class="text-muted-foreground">No Cover</span>
              </div>
            {/if}
          </div>
          <div class="flex min-w-0 flex-col overflow-hidden">
            <span class="truncate text-base font-medium"
              >{$trackStore.currentTrack.title}</span
            >
            <span class="text-muted-foreground truncate text-sm">
              {$trackStore.currentTrack.artist}
            </span>
          </div>
        {:else}
          <div
            class="bg-muted flex h-18 w-18 flex-shrink-0 items-center justify-center rounded-md"
          >
            <span class="text-muted-foreground text-xs">No Track</span>
          </div>
          <div class="flex min-w-0 flex-col overflow-hidden">
            <span class="text-muted-foreground text-sm">Not Playing</span>
          </div>
        {/if}
      </div>

      <!-- Controls -->
      <div class="flex flex-1 flex-col items-center justify-center">
        <!-- Control Buttons Row -->
        <div class="flex items-center space-x-2">
          <!-- Shuffle Button -->
          <Button
            variant="ghost"
            size="icon"
            class="h-9 w-9 {$trackStore.is_shuffle ? 'bg-accent/10' : ''}"
            on:click={() => trackStore.toggleShuffle()}
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
            class="h-9 w-9 {isRepeat ? 'bg-accent/10' : ''}"
            on:click={() => {
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
            class="h-10 w-10"
            on:click={() => trackStore.previousTrack()}
            aria-label="Previous Track"
            disabled={!$trackStore.currentTrack}
          >
            <SkipBack class="h-6 w-6" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-12 w-12"
            on:click={() => {
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
            class="h-10 w-10"
            on:click={() => trackStore.nextTrack()}
            aria-label="Next Track"
            disabled={!$trackStore.currentTrack}
          >
            <SkipForward class="h-6 w-6" />
          </Button>

          <!-- Volume Controls -->
          <Button
            variant="ghost"
            size="icon"
            class="h-9 w-9"
            on:click={() => {
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
          <div class="relative w-32">
            <Slider
              bind:value={volumeValue}
              onValueChange={handleVolumeChange}
              onValueCommit={handleVolumeCommit}
              onInput={handleVolumeInput}
              max={1}
              step={0.01}
              class="w-full"
            />
            {#if showVolumeFeedback}
              <div
                class="bg-primary text-primary-foreground absolute -top-7 left-1/2 -translate-x-1/2 rounded px-2 py-1 text-xs font-medium transition-opacity"
                style="opacity: {showVolumeFeedback ? '1' : '0'}"
              >
                {volumeFeedbackValue}%
              </div>
            {/if}
          </div>
        </div>

        <!-- Progress Slider Row -->
        <div class="mt-2 flex w-full max-w-lg items-center space-x-2">
          <span class="text-muted-foreground w-10 text-right text-xs">
            {formatTime(currentTime)}
          </span>
          <Slider
            bind:value={progressValue}
            onValueChange={handleProgressChange}
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
        <!-- Mobile Menu Button -->
        <Button
          variant="ghost"
          size="icon"
          class="desktop:hidden ml-2"
          on:click={toggleMenu}
          aria-label="Open menu"
        >
          <Menu class="h-5 w-5" />
        </Button>
      </div>
    </div>
  </Card>
</div>
