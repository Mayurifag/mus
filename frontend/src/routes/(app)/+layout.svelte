<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { playerStore } from "$lib/stores/playerStore";
  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import { savePlayerState, getStreamUrl } from "$lib/services/apiClient";
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
    }
  });

  // Clean up event source on component destroy
  onDestroy(() => {
    if (eventSource) {
      eventSource.close();
    }

    // Remove event listener - only in browser
    if (browser && document) {
      document.body.removeEventListener("toggle-sheet", handleToggleMenu);
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
      // Otherwise go to next track
      trackStore.nextTrack();
    }
  }

  // Save player state to the backend
  function debouncedSavePlayerState() {
    if (saveStateDebounceTimer) {
      clearTimeout(saveStateDebounceTimer);
    }

    saveStateDebounceTimer = setTimeout(() => {
      if ($playerStore.currentTrack) {
        savePlayerState({
          current_track_id: $playerStore.currentTrack.id,
          progress_seconds: $playerStore.currentTime,
          volume_level: $playerStore.volume,
          is_muted: $playerStore.isMuted,
          is_shuffle: $playerStore.is_shuffle,
          is_repeat: $playerStore.is_repeat,
        });
      }
    }, 1000); // Save after 1 second of no changes
  }

  // React to store changes
  $: if (audio && $playerStore.currentTrack) {
    const streamUrl = getStreamUrl($playerStore.currentTrack.id);
    if (audio.src !== streamUrl) {
      audio.src = streamUrl;
      audio.load();
      trackLoaded = false;
    }
  }

  $: if (audio && $playerStore.isPlaying && trackLoaded) {
    audio.play().catch((error) => {
      console.error("Error playing audio:", error);
      playerStore.pause();
    });
  } else if (audio && !$playerStore.isPlaying) {
    audio.pause();
  }

  $: if (audio) {
    audio.volume = $playerStore.isMuted ? 0 : $playerStore.volume;
  }

  $: if (
    $playerStore.currentTrack &&
    ($playerStore.isPlaying ||
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
  <div class="flex h-screen flex-col">
    <main class="flex flex-1 overflow-hidden">
      <div class="flex-1 overflow-y-auto p-4">
        <slot />
      </div>

      <!-- Desktop Sidebar - visible on md screens and up -->
      <div class="hidden shrink-0 md:block">
        <RightSidebar />
      </div>

      <!-- Mobile Sidebar Sheet -->
      <Sheet.Content side="right" class="w-64">
        <Sheet.Header>
          <Sheet.Title>Menu</Sheet.Title>
          <Sheet.Description>Navigation and library controls</Sheet.Description>
        </Sheet.Header>
        <RightSidebar />
      </Sheet.Content>
    </main>

    <PlayerFooter />

    <audio
      bind:this={audio}
      on:timeupdate={handleTimeUpdate}
      on:loadedmetadata={handleLoadedMetadata}
      on:ended={handleEnded}
      preload="auto"
    ></audio>
  </div>
</Sheet.Root>
