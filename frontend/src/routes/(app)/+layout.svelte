<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { playerStore } from "$lib/stores/playerStore";
  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import { savePlayerState, getStreamUrl } from "$lib/services/apiClient";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import type { Track } from "$lib/types";

  export let data: {
    tracks: Track[];
    playerState: null | {
      current_track_id: number | null;
      progress_seconds: number;
      volume_level: number;
      is_muted: boolean;
    };
  };

  let audio: HTMLAudioElement;
  let trackLoaded = false;
  let saveStateDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  let eventSource: EventSource | null = null;

  // Initialize stores with server-loaded data
  onMount(() => {
    // Initialize tracks
    if (data.tracks && data.tracks.length > 0) {
      trackStore.setTracks(data.tracks);
    }

    // Initialize player state if available
    if (data.playerState) {
      const { current_track_id, progress_seconds, volume_level, is_muted } =
        data.playerState;

      // Set volume and mute state
      playerStore.setVolume(volume_level);
      if (is_muted) {
        playerStore.toggleMute();
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
        }
      }
    }

    // Initialize SSE connection for track updates
    eventSource = initEventHandlerService();
  });

  // Clean up event source on component destroy
  onDestroy(() => {
    if (eventSource) {
      eventSource.close();
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
    trackStore.nextTrack();
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

  // Save state when relevant values change
  $: if ($playerStore.currentTrack || $playerStore.currentTime > 0) {
    debouncedSavePlayerState();
  }
</script>

<slot />

<PlayerFooter />

<audio
  bind:this={audio}
  on:timeupdate={handleTimeUpdate}
  on:loadedmetadata={handleLoadedMetadata}
  on:ended={handleEnded}
  preload="auto"
></audio>
