<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";
  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import { sendPlayerStateBeacon as apiSendPlayerStateBeacon } from "$lib/services/apiClient";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import { AudioService } from "$lib/services/AudioService";
  import * as Sheet from "$lib/components/ui/sheet";
  import type { Track } from "$lib/types";
  import { browser } from "$app/environment";
  import { Toaster } from "$lib/components/ui/sonner";
  import type { LayoutData } from "./$types";

  export let data: LayoutData;

  let audio: HTMLAudioElement;
  let audioService: AudioService | undefined = undefined;
  let eventSource: EventSource | null = null;
  let sheetOpen = false;
  let lastPlayerStateSaveTime = 0;
  let lastCurrentTrackId: number | null = null;
  let isInitialLoad = true;

  onMount(async () => {
    trackStore.setTracks(data.tracks);

    // Initialize audio service first
    if (audio) {
      audioService = new AudioService(audio, () => trackStore.nextTrack());
      audioServiceStore.set(audioService);
    }

    if (data.playerState) {
      const {
        current_track_id,
        progress_seconds,
        volume_level,
        is_muted,
        is_shuffle,
        is_repeat,
      } = data.playerState;

      // Initialize AudioService state
      if (audioService) {
        audioService.initializeState(volume_level, is_muted, is_repeat);
      }

      trackStore.setShuffle(is_shuffle);

      if (current_track_id !== null) {
        const trackIndex = data.tracks.findIndex(
          (track: Track) => track.id === current_track_id,
        );
        if (trackIndex >= 0) {
          trackStore.setCurrentTrackIndex(trackIndex);
          if (audioService) {
            audioService.setCurrentTime(progress_seconds);
          }
        }
      }
    } else {
      if (data.tracks.length > 0) {
        trackStore.setCurrentTrackIndex(0);
      }
    }

    eventSource = initEventHandlerService();

    if (browser) {
      document.body.addEventListener("toggle-sheet", handleToggleMenu);
      window.addEventListener("beforeunload", handleBeforeUnload);
      document.addEventListener("visibilitychange", handleVisibilityChange);
    }

    if (
      $trackStore.currentTrackIndex !== null &&
      $trackStore.tracks.length > 0
    ) {
      const currentTrack = $trackStore.tracks[$trackStore.currentTrackIndex];
      if (currentTrack) {
        await tick();
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

    // Mark initial load as complete
    isInitialLoad = false;
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

  function savePlayerState() {
    if (!$trackStore.currentTrack || !audioService) return null;
    const playerStateDto = {
      current_track_id: $trackStore.currentTrack.id,
      progress_seconds: audioService.currentTime,
      volume_level: audioService.volume,
      is_muted: audioService.isMuted,
      is_shuffle: $trackStore.is_shuffle,
      is_repeat: audioService.isRepeat,
    };

    apiSendPlayerStateBeacon(playerStateDto);
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

  $: if (
    audioService &&
    $trackStore.currentTrack &&
    $trackStore.currentTrack.id !== lastCurrentTrackId
  ) {
    console.log(
      "$: Track changed - updating audio source",
      $trackStore.currentTrack.id,
    );
    // Only auto-play if this is not the initial page load
    const shouldAutoPlay = !isInitialLoad;
    audioService.updateAudioSource($trackStore.currentTrack, shouldAutoPlay);
    lastCurrentTrackId = $trackStore.currentTrack.id;
  }

  $: if ($trackStore.currentTrack && audioService && audioService.isPlaying) {
    const now = Date.now();
    if (now - lastPlayerStateSaveTime > 5000) {
      savePlayerState();
      lastPlayerStateSaveTime = now;
    }
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
  <PlayerFooter audioService={$audioServiceStore} />

  <audio bind:this={audio} preload="auto" id="mus-audio-element"></audio>
</Sheet.Root>
