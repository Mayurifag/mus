<script lang="ts">
  import { Card } from "$lib/components/ui/card";
  import { Button } from "$lib/components/ui/button";
  import { Slider } from "$lib/components/ui/slider";
  import { trackStore } from "$lib/stores/trackStore";
  import type { AudioService } from "$lib/services/AudioService";
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

  // Reactive volume feedback value
  let volumeFeedbackValue = $derived(
    isMuted ? 0 : Math.round((volumeValue[0] || 1) * 100),
  );

  // Sync local state with AudioService stores when they change
  $effect(() => {
    if (audioService?.isPlayingStore) {
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log("PlayerFooter $effect: Subscribing to isPlayingStore");
      const unsubscribe = audioService.isPlayingStore.subscribe((playing) => {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log("PlayerFooter $effect: isPlaying state changed:", playing);
        isPlaying = playing;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.isMutedStore) {
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log("PlayerFooter $effect: Subscribing to isMutedStore");
      const unsubscribe = audioService.isMutedStore.subscribe((muted) => {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log("PlayerFooter $effect: isMuted state changed:", muted);
        isMuted = muted;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.currentTimeStore) {
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log("PlayerFooter $effect: Subscribing to currentTimeStore");
      const unsubscribe = audioService.currentTimeStore.subscribe((time) => {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log(
          "PlayerFooter $effect: currentTime changed:",
          time,
          "User dragging progress:",
          isUserDraggingProgress,
        );
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
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log("PlayerFooter $effect: Subscribing to durationStore");
      const unsubscribe = audioService.durationStore.subscribe((dur) => {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log("PlayerFooter $effect: duration changed:", dur);
        duration = dur;
      });
      return unsubscribe;
    }
  });

  $effect(() => {
    if (audioService?.volumeStore) {
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log("PlayerFooter $effect: Subscribing to volumeStore");
      const unsubscribe = audioService.volumeStore.subscribe((volume) => {
        // For AI assistant: Debug logging - do not remove this log, user intended to have it
        console.log(
          "PlayerFooter $effect: volume changed:",
          volume,
          "User dragging volume:",
          isUserDraggingVolume,
        );
        if (!isUserDraggingVolume) {
          volumeValue = [volume];
        }
      });
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
    // For AI assistant: Debug logging - do not remove this log, user intended to have it
    console.log(
      "PlayerFooter $effect: Setting up volume feedback timer cleanup",
    );

    return () => {
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log("PlayerFooter $effect: Cleaning up volume feedback timer");
      if (volumeFeedbackTimer) {
        clearTimeout(volumeFeedbackTimer);
      }
    };
  });
</script>

<div class="bg-card fixed right-0 bottom-0 left-0 z-50 border-t">
  <Card class="rounded-none border-0 shadow-none">
    <div class="flex h-20 items-center px-4">
      <!-- Track Info -->
      <div class="flex w-1/3 items-center space-x-4">
        {#if $trackStore.currentTrack}
          <div class="h-14 w-14 overflow-hidden rounded-md">
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
          <div class="flex flex-col overflow-hidden">
            <span class="truncate text-sm font-medium"
              >{$trackStore.currentTrack.title}</span
            >
            <span class="text-muted-foreground truncate text-xs">
              {$trackStore.currentTrack.artist}
            </span>
          </div>
        {:else}
          <div
            class="bg-muted flex h-14 w-14 items-center justify-center rounded-md"
          >
            <span class="text-muted-foreground text-xs">No Track</span>
          </div>
          <div class="flex flex-col overflow-hidden">
            <span class="text-muted-foreground text-sm">Not Playing</span>
          </div>
        {/if}
      </div>

      <!-- Controls -->
      <div class="flex w-1/3 flex-col items-center justify-center">
        <div class="flex items-center space-x-2">
          <!-- Shuffle Button -->
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8"
            on:click={() => trackStore.toggleShuffle()}
            aria-label="Toggle Shuffle"
            aria-pressed={$trackStore.is_shuffle}
          >
            <Shuffle
              class="h-5 w-5"
              color={$trackStore.is_shuffle ? "var(--accent)" : "currentColor"}
            />
          </Button>

          <!-- Repeat Button -->
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8"
            on:click={() => {
              if (audioService) {
                audioService.toggleRepeat();
              }
            }}
            aria-label="Toggle Repeat"
            aria-pressed={audioService?.isRepeat}
          >
            {#if audioService?.isRepeat}
              <Repeat1 class="h-5 w-5" color="var(--accent)" />
            {:else}
              <Repeat class="h-5 w-5" />
            {/if}
          </Button>

          <Button
            variant="ghost"
            size="icon"
            class="h-9 w-9"
            on:click={() => trackStore.previousTrack()}
            aria-label="Previous Track"
            disabled={!$trackStore.currentTrack}
          >
            <SkipBack class="h-5 w-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-10 w-10"
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
              <Pause class="h-6 w-6" />
            {:else}
              <Play class="h-6 w-6" />
            {/if}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-9 w-9"
            on:click={() => trackStore.nextTrack()}
            aria-label="Next Track"
            disabled={!$trackStore.currentTrack}
          >
            <SkipForward class="h-5 w-5" />
          </Button>
        </div>
        <div class="mt-1 flex w-full max-w-md items-center space-x-2 px-4">
          <span class="text-muted-foreground w-10 text-right text-xs">
            {formatTime(currentTime)}
          </span>
          <Slider
            bind:value={progressValue}
            onValueChange={handleProgressChange}
            on:valuecommit={handleProgressCommit}
            on:input={handleProgressInput}
            max={duration || 100}
            step={1}
            class="flex-1 cursor-pointer"
            disabled={!$trackStore.currentTrack}
          />
          <span class="text-muted-foreground w-10 text-xs">
            {formatTime(duration)}
          </span>
        </div>
      </div>

      <!-- Volume Controls and Additional Controls -->
      <div class="flex w-1/3 items-center justify-end space-x-2 pr-4">
        <!-- Volume Controls with Visual Feedback -->
        <div class="relative flex items-center space-x-1">
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8"
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
              on:valuecommit={handleVolumeCommit}
              on:input={handleVolumeInput}
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

        <!-- Mobile Menu Button -->
        <Button
          variant="ghost"
          size="icon"
          class="ml-2 md:hidden"
          on:click={toggleMenu}
          aria-label="Open menu"
        >
          <Menu class="h-5 w-5" />
        </Button>
      </div>
    </div>
  </Card>
</div>
