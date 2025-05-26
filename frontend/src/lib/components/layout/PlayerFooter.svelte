<script lang="ts">
  import { Card } from "$lib/components/ui/card";
  import { Button } from "$lib/components/ui/button";
  import { Slider } from "$lib/components/ui/slider";
  import { playerStore } from "$lib/stores/playerStore";
  import { trackStore } from "$lib/stores/trackStore";
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
  } from "lucide-svelte";
  import { browser } from "$app/environment";
  import { onMount } from "svelte";

  function formatTime(seconds: number): string {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function handleSeek(value: number[]) {
    if (value.length > 0) {
      playerStore.setCurrentTime(value[0]);
    }
  }

  // Volume feedback variables
  let showVolumeFeedback = false;
  let volumeFeedbackValue = 0;
  let volumeFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

  function handleVolumeChange(value: number[]) {
    if (value.length > 0) {
      playerStore.setVolume(value[0]);

      // Show volume feedback
      volumeFeedbackValue = Math.round(value[0] * 100);
      showVolumeFeedback = true;

      // Clear existing timer if any
      if (volumeFeedbackTimer) {
        clearTimeout(volumeFeedbackTimer);
      }

      // Hide feedback after 1.5 seconds
      volumeFeedbackTimer = setTimeout(() => {
        showVolumeFeedback = false;
      }, 200);
    }
  }

  // Create a function to dispatch a custom event to toggle the sidebar
  function toggleMenu() {
    if (browser) {
      const event = new CustomEvent("toggle-sheet");
      document.body.dispatchEvent(event);
    }
  }

  // Update volume feedback when mute is toggled
  $: if ($playerStore.isMuted) {
    volumeFeedbackValue = 0;
  } else {
    volumeFeedbackValue = Math.round($playerStore.volume * 100);
  }

  // Clean up timer when component is destroyed
  onMount(() => {
    return () => {
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
        {#if $playerStore.currentTrack}
          <div class="h-14 w-14 overflow-hidden rounded-md">
            {#if $playerStore.currentTrack.has_cover && $playerStore.currentTrack.cover_original_url}
              <img
                src={$playerStore.currentTrack.cover_original_url}
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
              >{$playerStore.currentTrack.title}</span
            >
            <span class="text-muted-foreground truncate text-xs">
              {$playerStore.currentTrack.artist}
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
          <Button
            variant="ghost"
            size="icon"
            class="h-9 w-9"
            on:click={() => trackStore.previousTrack()}
            aria-label="Previous Track"
            disabled={!$playerStore.currentTrack}
          >
            <SkipBack class="h-5 w-5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-10 w-10"
            on:click={() => playerStore.togglePlayPause()}
            aria-label={$playerStore.isPlaying ? "Pause" : "Play"}
            disabled={!$playerStore.currentTrack}
          >
            {#if $playerStore.isPlaying}
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
            disabled={!$playerStore.currentTrack}
          >
            <SkipForward class="h-5 w-5" />
          </Button>
        </div>
        <div class="mt-1 flex w-full max-w-md items-center space-x-2 px-4">
          <span class="text-muted-foreground w-10 text-right text-xs">
            {formatTime($playerStore.currentTime)}
          </span>
          <Slider
            value={[$playerStore.currentTime]}
            onValueChange={handleSeek}
            max={$playerStore.duration || 100}
            step={1}
            class="flex-1"
            disabled={!$playerStore.currentTrack}
          />
          <span class="text-muted-foreground w-10 text-xs">
            {formatTime($playerStore.duration)}
          </span>
        </div>
      </div>

      <!-- Volume Controls and Additional Controls -->
      <div class="flex w-1/3 items-center justify-end space-x-2 pr-4">
        <!-- Shuffle Button -->
        <Button
          variant="ghost"
          size="icon"
          class="h-8 w-8"
          on:click={() => playerStore.toggleShuffle()}
          aria-label="Toggle Shuffle"
          aria-pressed={$playerStore.is_shuffle}
        >
          <Shuffle
            class="h-5 w-5"
            color={$playerStore.is_shuffle ? "var(--accent)" : "currentColor"}
          />
        </Button>

        <!-- Repeat Button -->
        <Button
          variant="ghost"
          size="icon"
          class="h-8 w-8"
          on:click={() => playerStore.toggleRepeat()}
          aria-label="Toggle Repeat"
          aria-pressed={$playerStore.is_repeat}
        >
          {#if $playerStore.is_repeat}
            <Repeat1 class="h-5 w-5" color="var(--accent)" />
          {:else}
            <Repeat class="h-5 w-5" />
          {/if}
        </Button>

        <!-- Volume Controls with Visual Feedback -->
        <div class="relative flex items-center space-x-1">
          <Button
            variant="ghost"
            size="icon"
            class="h-8 w-8"
            on:click={() => playerStore.toggleMute()}
            aria-label={$playerStore.isMuted ? "Unmute" : "Mute"}
          >
            {#if $playerStore.isMuted}
              <VolumeX class="h-5 w-5" />
            {:else}
              <Volume2 class="h-5 w-5" />
            {/if}
          </Button>
          <div class="relative w-32">
            <Slider
              value={[$playerStore.volume]}
              onValueChange={handleVolumeChange}
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
