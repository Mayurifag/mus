<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { playerStore } from "$lib/stores/playerStore";
  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import { sendPlayerStateBeacon as apiSendPlayerStateBeacon } from "$lib/services/apiClient";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import { AudioService } from "$lib/services/AudioService";
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
  let audioService: AudioService;
  let eventSource: EventSource | null = null;
  let sheetOpen = false;
  let lastPlayerStateSaveTime = 0;
  let currentTrackIdForWebpageTitle: number | null = null;

  onMount(async () => {
    // Initialize stores from data prop
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
          trackStore.setCurrentTrackIndex(trackIndex);
          playerStore.setCurrentTime(progress_seconds);
          playerStore.pause();
        }
      }
    } else {
      trackStore.setCurrentTrackIndex(0);
      playerStore.pause();
    }

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

    // Initialize AudioService when audio element is available
    if (audio) {
      audioService = new AudioService(audio, playerStore, trackStore);
    }
  });

  // Clean up event source on component destroy
  onDestroy(() => {
    if (eventSource) {
      eventSource.close();
    }

    // Clean up AudioService
    if (audioService) {
      audioService.destroy();
    }

    // Remove event listeners - only in browser
    if (browser && document) {
      document.body.removeEventListener("toggle-sheet", handleToggleMenu);
      window.removeEventListener("beforeunload", handleBeforeUnload);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    }
  });

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

  // React to store changes using AudioService
  $: if (audioService && $playerStore.currentTrack) {
    console.log("$: if (audioService && $playerStore.currentTrack)");
    audioService.updateAudioSource(
      $playerStore.currentTrack,
      $playerStore.isPlaying,
    );
  }

  $: if (audioService && $playerStore.isPlaying) {
    console.log("$: if (audioService && $playerStore.isPlaying)");
    audioService.play();
  } else if (audioService && !$playerStore.isPlaying) {
    audioService.pause();
  }

  $: if (audioService) {
    console.log("$: if (audioService)");
    audioService.setVolume($playerStore.volume, $playerStore.isMuted);
  }

  // Sync store currentTime changes to audio (for seeking)
  $: if (audioService && $playerStore.currentTime !== undefined) {
    console.log(
      "$: if (audioService && $playerStore.currentTime !== undefined)",
    );
    audioService.setCurrentTime($playerStore.currentTime);
  }

  // Save player state on any relevant changes
  $: if ($playerStore.currentTrack && $playerStore.isPlaying) {
    console.log("$: if ($playerStore.currentTrack && $playerStore.isPlaying)");
    const now = Date.now();
    if (now - lastPlayerStateSaveTime > 5000) {
      savePlayerState();
      lastPlayerStateSaveTime = now;
    }
  }

  $: if (
    $playerStore.currentTrack &&
    browser &&
    $playerStore.currentTrack.id !== currentTrackIdForWebpageTitle
  ) {
    console.log(
      "$: if ($playerStore.currentTrack && browser && $playerStore.currentTrack.id !== currentTrackIdForWebpageTitle)",
    );
    document.title = `${$playerStore.currentTrack.artist} - ${$playerStore.currentTrack.title}`;
    currentTrackIdForWebpageTitle = $playerStore.currentTrack.id;
  }

  function handleToggleMenu() {
    sheetOpen = !sheetOpen;
  }
</script>

<Sheet.Root bind:open={sheetOpen}>
  <!-- Main content area that uses full viewport scrolling -->
  <main class="min-h-screen pb-20 pr-0 md:pr-64">
    <div class="p-4">
      <slot />
    </div>
  </main>

  <Toaster position="top-left" />

  <!-- Desktop Sidebar - positioned fixed on the right -->
  <aside class="fixed bottom-20 right-0 top-0 hidden w-64 md:block">
    <RightSidebar />
  </aside>

  <!-- Mobile Sidebar Sheet -->
  <Sheet.Content side="right" class="w-64">
    <RightSidebar />
  </Sheet.Content>

  <!-- Fixed Player Footer -->
  <PlayerFooter />

  <audio bind:this={audio} preload="auto"></audio>
</Sheet.Root>
