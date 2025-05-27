<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { playerStore } from "$lib/stores/playerStore";
  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import { savePlayerStateAsync, getStreamUrl } from "$lib/services/apiClient";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import * as Sheet from "$lib/components/ui/sheet";
  import type { Track } from "$lib/types";
  import { browser } from "$app/environment";

  export let data: {
    tracks: Track[];
    playerState: null | {
      current_track_id: number | null;
      progress_seconds: number;
      volume_level: number;
      is_muted: boolean;
      is_shuffle: boolean;
      is_repeat: boolean;
    };
  };

  let audio: HTMLAudioElement;
  let trackLoaded = false;
  let saveStateDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  let eventSource: EventSource | null = null;
  let sheetOpen = false;
  let shouldAutoPlay = false; // Track intent to auto-play when track loads

  function handleToggleMenu() {
    sheetOpen = !sheetOpen;
  }

  // Initialize stores with server-loaded data
  onMount(() => {
    // Initialize tracks
    if (data.tracks && data.tracks.length > 0) {
      trackStore.setTracks(data.tracks);
    }

    let trackSelected = false;

    // Initialize player state if available
    if (data.playerState) {
      const {
        current_track_id,
        progress_seconds,
        volume_level,
        is_muted,
        is_shuffle,
        is_repeat,
      } = data.playerState;

      // Set volume and mute state
      playerStore.setVolume(volume_level);
      if (is_muted) {
        playerStore.setMuted(true);
      }

      // Set shuffle and repeat state
      if (is_shuffle) {
        playerStore.update((state) => ({ ...state, is_shuffle: true }));
      }

      if (is_repeat) {
        playerStore.update((state) => ({ ...state, is_repeat: true }));
      }

      // Set current track if exists
      if (current_track_id !== null) {
        const trackIndex = data.tracks.findIndex(
          (track: Track) => track.id === current_track_id,
        );
        if (trackIndex >= 0) {
          trackStore.setCurrentTrackIndex(trackIndex);
          // Set progress
          playerStore.setCurrentTime(progress_seconds);
          trackSelected = true;
        }
      }
    }

    // If no track was selected and we have tracks, select the first one
    if (!trackSelected && data.tracks && data.tracks.length > 0) {
      trackStore.setCurrentTrackIndex(0);
      playerStore.pause();
    }

    // Initialize SSE connection for track updates
    eventSource = initEventHandlerService();

    // Add event listener for menu toggle - only in browser
    if (browser) {
      document.body.addEventListener("toggle-sheet", handleToggleMenu);

      // Add event listeners for player state persistence on page unload
      window.addEventListener("beforeunload", handleBeforeUnload);
      document.addEventListener("visibilitychange", handleVisibilityChange);
    }
  });

  // Clean up event source on component destroy
  onDestroy(() => {
    if (eventSource) {
      eventSource.close();
    }

    if (saveStateDebounceTimer) {
      clearTimeout(saveStateDebounceTimer);
    }

    // Remove event listeners - only in browser
    if (browser && document) {
      document.body.removeEventListener("toggle-sheet", handleToggleMenu);
      window.removeEventListener("beforeunload", handleBeforeUnload);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    }
  });

  // Handle audio events
  function handleTimeUpdate() {
    if (audio && !isNaN(audio.currentTime)) {
      playerStore.setCurrentTime(audio.currentTime);
    }
  }

  function handleLoadedMetadata() {
    if (audio && !isNaN(audio.duration)) {
      playerStore.update((state) => ({ ...state, duration: audio.duration }));
      trackLoaded = true;

      // If we had a stored time position, seek to it
      if ($playerStore.currentTime > 0) {
        audio.currentTime = $playerStore.currentTime;
      }

      // If we should auto-play (either from store state or saved intent), start playback
      if ($playerStore.isPlaying || shouldAutoPlay) {
        shouldAutoPlay = false; // Clear the flag
        audio.play().catch((error) => {
          console.error("Error playing audio after metadata loaded:", error);
          // Don't pause on AbortError - it's expected when changing tracks
          if (error.name !== "AbortError") {
            playerStore.pause();
          }
        });
      }
    }
  }

  function handleEnded() {
    // If repeat is enabled, restart the current track
    if ($playerStore.is_repeat) {
      if (audio) {
        audio.currentTime = 0;
        audio.play().catch((error) => {
          console.error("Error replaying audio:", error);
          playerStore.pause();
        });
      }
    } else {
      // Otherwise go to next track with shuffle support
      trackStore.nextTrack();
    }
  }

  function handleError() {
    console.error("Audio playback error occurred");
    playerStore.pause();
  }

  // Construct PlayerStateDTO from current store state
  function constructPlayerStateDTO() {
    if (!$playerStore.currentTrack) return null;

    return {
      current_track_id: $playerStore.currentTrack.id,
      progress_seconds: $playerStore.currentTime,
      volume_level: $playerStore.volume,
      is_muted: $playerStore.isMuted,
      is_shuffle: $playerStore.is_shuffle,
      is_repeat: $playerStore.is_repeat,
    };
  }

  // Save player state to the backend
  function debouncedSavePlayerState() {
    if (saveStateDebounceTimer) {
      clearTimeout(saveStateDebounceTimer);
    }

    saveStateDebounceTimer = setTimeout(() => {
      const playerStateDto = constructPlayerStateDTO();
      if (playerStateDto) {
        // Validate the data before sending
        if (playerStateDto.progress_seconds < 0) {
          playerStateDto.progress_seconds = 0;
        }
        if (playerStateDto.volume_level < 0) {
          playerStateDto.volume_level = 0;
        }
        if (playerStateDto.volume_level > 1) {
          playerStateDto.volume_level = 1;
        }

        savePlayerStateAsync(playerStateDto);
      }
    }, 1000); // Save after 1 second of no changes
  }

  // Send player state via navigator.sendBeacon on page unload
  function sendPlayerStateBeacon() {
    const playerStateDto = constructPlayerStateDTO();
    if (playerStateDto && browser && navigator.sendBeacon) {
      // Cancel any pending debounced save since we're sending immediately
      if (saveStateDebounceTimer) {
        clearTimeout(saveStateDebounceTimer);
        saveStateDebounceTimer = null;
      }

      // Validate the data before sending
      if (playerStateDto.progress_seconds < 0) {
        playerStateDto.progress_seconds = 0;
      }
      if (playerStateDto.volume_level < 0) {
        playerStateDto.volume_level = 0;
      }
      if (playerStateDto.volume_level > 1) {
        playerStateDto.volume_level = 1;
      }

      const apiBaseUrl = import.meta.env.DEV
        ? "http://localhost:8000/api/v1"
        : "/api/v1";
      const url = `${apiBaseUrl}/player/state`;

      // Create a proper Blob with correct Content-Type for JSON
      const blob = new Blob([JSON.stringify(playerStateDto)], {
        type: "application/json",
      });

      navigator.sendBeacon(url, blob);
    }
  }

  // Handle page unload events
  function handleBeforeUnload() {
    sendPlayerStateBeacon();
  }

  // Handle visibility change events
  function handleVisibilityChange() {
    if (document.visibilityState === "hidden") {
      sendPlayerStateBeacon();
    }
  }

  // React to store changes
  $: if (audio && $playerStore.currentTrack) {
    const streamUrl = getStreamUrl($playerStore.currentTrack.id);
    if (audio.src !== streamUrl) {
      // Set auto-play intent if we should be playing
      shouldAutoPlay = $playerStore.isPlaying;
      audio.src = streamUrl;
      audio.load();
      trackLoaded = false;
    }
  }

  $: if (audio && $playerStore.isPlaying && trackLoaded) {
    audio.play().catch((error) => {
      console.error("Error playing audio:", error);
      // Don't pause on AbortError - it's expected when changing tracks
      if (error.name !== "AbortError") {
        playerStore.pause();
      }
    });
  } else if (audio && !$playerStore.isPlaying) {
    audio.pause();
  }

  $: if (audio) {
    audio.volume = $playerStore.isMuted ? 0 : $playerStore.volume;
  }

  // Sync store currentTime changes to audio (for seeking)
  $: if (audio && trackLoaded && $playerStore.currentTime !== undefined) {
    // Only seek if there's a significant difference (more than 1 second)
    // This prevents infinite loops from natural timeupdate events
    const timeDiff = Math.abs(audio.currentTime - $playerStore.currentTime);
    if (timeDiff > 1) {
      audio.currentTime = $playerStore.currentTime;
    }
  }

  // Trigger debounced save on any relevant player state changes
  $: if (
    $playerStore.currentTrack &&
    ($playerStore.currentTime ||
      $playerStore.volume ||
      $playerStore.isMuted ||
      $playerStore.is_shuffle ||
      $playerStore.is_repeat)
  ) {
    debouncedSavePlayerState();
  }

  $: if (browser) {
    document.title = $playerStore.currentTrack
      ? `${$playerStore.currentTrack.artist} - ${$playerStore.currentTrack.title}`
      : "Mus";
  }
</script>

<Sheet.Root bind:open={sheetOpen}>
  <!-- Main content area that uses full viewport scrolling -->
  <main class="min-h-screen pr-0 pb-20 md:pr-64">
    <div class="p-4">
      <slot />
    </div>
  </main>

  <!-- Desktop Sidebar - positioned fixed on the right -->
  <aside class="fixed top-0 right-0 bottom-20 hidden w-64 md:block">
    <RightSidebar />
  </aside>

  <!-- Mobile Sidebar Sheet -->
  <Sheet.Content side="right" class="w-64">
    <Sheet.Header>
      <Sheet.Title>Menu</Sheet.Title>
      <Sheet.Description>Navigation and library controls</Sheet.Description>
    </Sheet.Header>
    <RightSidebar />
  </Sheet.Content>

  <!-- Fixed Player Footer -->
  <PlayerFooter />

  <audio
    bind:this={audio}
    on:timeupdate={handleTimeUpdate}
    on:loadedmetadata={handleLoadedMetadata}
    on:ended={handleEnded}
    on:error={handleError}
    preload="auto"
  ></audio>
</Sheet.Root>
