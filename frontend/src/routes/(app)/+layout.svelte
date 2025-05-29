<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { playerStore } from "$lib/stores/playerStore";
  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import {
    getStreamUrl,
    sendPlayerStateBeacon as apiSendPlayerStateBeacon,
  } from "$lib/services/apiClient";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import * as Sheet from "$lib/components/ui/sheet";
  import type { Track } from "$lib/types";
  import { browser } from "$app/environment";
  import { Toaster } from "$lib/components/ui/sonner";

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
  // let trackLoaded = false;
  let eventSource: EventSource | null = null;
  let sheetOpen = false;
  let shouldAutoPlay = false;

  trackStore.setTracks(data.tracks);

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
    playerStore.setShuffle(is_shuffle);
    playerStore.setRepeat(is_repeat);

    // Set current track if exists
    if (current_track_id !== null) {
      const trackIndex = data.tracks.findIndex(
        (track: Track) => track.id === current_track_id,
      );
      if (trackIndex >= 0) {
        console.log(
          "setting current track index",
          trackIndex,
          progress_seconds,
        );
        trackStore.setCurrentTrackIndex(trackIndex);
        playerStore.setCurrentTime(progress_seconds);
        playerStore.pause();
      }
    }
  } else {
    trackStore.setCurrentTrackIndex(0);
    playerStore.pause();
  }

  onMount(async () => {
    // Initialize SSE connection for track updates
    eventSource = initEventHandlerService();

    // Add event listener for menu toggle - only in browser
    if (browser) {
      document.body.addEventListener("toggle-sheet", handleToggleMenu);
      window.addEventListener("beforeunload", handleBeforeUnload);
      document.addEventListener("visibilitychange", handleVisibilityChange);
    }

    // Perform initial scroll to current track
    if (
      $trackStore.currentTrackIndex !== null &&
      $trackStore.tracks.length > 0
    ) {
      const currentTrack = $trackStore.tracks[$trackStore.currentTrackIndex];
      if (currentTrack) {
        const trackElement = document.getElementById(
          `track-item-${currentTrack.id}`,
        );
        if (trackElement) {
          trackElement.scrollIntoView({
            behavior: "auto",
            block: "center",
          });
        }
      }
    }

    // Set initial volume when audio element is available
    if (audio) {
      audio.volume = $playerStore.isMuted ? 0 : $playerStore.volume;
    }
  });

  // Clean up event source on component destroy
  onDestroy(() => {
    if (eventSource) {
      eventSource.close();
    }

    // Remove event listeners - only in browser
    if (browser && document) {
      document.body.removeEventListener("toggle-sheet", handleToggleMenu);
      window.removeEventListener("beforeunload", handleBeforeUnload);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    }
  });

  function handleTimeUpdate() {
    if (audio && !isNaN(audio.currentTime)) {
      playerStore.setCurrentTime(audio.currentTime);
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
  function savePlayerState() {
    const playerStateDto = constructPlayerStateDTO();
    if (playerStateDto && browser) {
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

      apiSendPlayerStateBeacon(playerStateDto);
    }
  }

  // Handle page unload events
  function handleBeforeUnload() {
    savePlayerState();
  }

  // Handle visibility change events
  function handleVisibilityChange() {
    if (document.visibilityState === "hidden") {
      savePlayerState();
    }
  }

  function updateAudioSource() {
    if (!audio || !$playerStore.currentTrack) return;

    const streamUrl = getStreamUrl($playerStore.currentTrack.id);
    if (audio.src !== streamUrl) {
      shouldAutoPlay = $playerStore.isPlaying;
      audio.src = streamUrl;
      audio.load();
    }
  }

  function handleCanPlay() {
    if (shouldAutoPlay) {
      audio.play().catch((error) => {
        console.error("Error auto-playing audio:", error);
        playerStore.pause();
      });
      shouldAutoPlay = false;
    }
  }

  // React to store changes
  $: if ($playerStore.currentTrack) {
    updateAudioSource();
  }

  $: if (audio && $playerStore.isPlaying) {
    console.log("Triggered reactive block: audio && $playerStore.isPlaying");
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
  $: if (audio && $playerStore.currentTime !== undefined) {
    console.log(
      "Triggered reactive block: audio && $playerStore.currentTime != undefined",
    );
    // Only seek if there's a significant difference (more than 1 second)
    // This prevents infinite loops from natural timeupdate events
    const timeDiff = Math.abs(audio.currentTime - $playerStore.currentTime);
    if (timeDiff > 1) {
      audio.currentTime = $playerStore.currentTime;
    }
  }

  // Save player state on any relevant changes
  $: if ($playerStore.currentTrack && $playerStore.isPlaying) {
    console.log(
      "Triggered reactive block: $playerStore.currentTrack && $playerStore.isPlaying",
    );
    // This reactive statement will trigger whenever any of these values change
    // We use void to explicitly indicate these are intentional side-effect expressions
    void $playerStore.currentTime;
    void $playerStore.volume;
    void $playerStore.isMuted;
    void $playerStore.is_shuffle;
    void $playerStore.is_repeat;
    savePlayerState();
  }

  $: if ($playerStore.currentTrack?.title && browser) {
    console.log(
      "Triggered reactive block: $playerStore.currentTrack?.title && browser",
    );
    document.title = `${$playerStore.currentTrack.artist} - ${$playerStore.currentTrack.title}`;
  }

  function handleToggleMenu() {
    sheetOpen = !sheetOpen;
  }
</script>

<Sheet.Root bind:open={sheetOpen}>
  <!-- Main content area that uses full viewport scrolling -->
  <main class="min-h-screen pr-0 pb-20 md:pr-64">
    <div class="p-4">
      <slot />
    </div>
  </main>

  <Toaster position="top-left" />

  <!-- Desktop Sidebar - positioned fixed on the right -->
  <aside class="fixed top-0 right-0 bottom-20 hidden w-64 md:block">
    <RightSidebar />
  </aside>

  <!-- Mobile Sidebar Sheet -->
  <Sheet.Content side="right" class="w-64">
    <RightSidebar />
  </Sheet.Content>

  <!-- Fixed Player Footer -->
  <PlayerFooter />

  <audio
    bind:this={audio}
    on:timeupdate={handleTimeUpdate}
    on:ended={handleEnded}
    on:error={handleError}
    on:canplay={handleCanPlay}
    preload="auto"
  ></audio>
</Sheet.Root>
