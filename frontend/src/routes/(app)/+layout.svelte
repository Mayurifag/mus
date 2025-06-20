<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { Snippet } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";

  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import { sendPlayerStateBeacon as apiSendPlayerStateBeacon } from "$lib/services/apiClient";
  import { authConfigStore } from "$lib/stores/authConfigStore";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import { AudioService } from "$lib/services/AudioService";
  import * as Sheet from "$lib/components/ui/sheet";
  import type { Track } from "$lib/types";
  import { browser } from "$app/environment";
  import { Toaster } from "$lib/components/ui/sonner";
  import type { LayoutData } from "./$types";

  // Import swipe functionality only on client side
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let swipe = $state<any>(undefined);

  let { data, children }: { data: LayoutData; children: Snippet } = $props();

  let audio: HTMLAudioElement;
  let audioService = $state<AudioService | undefined>(undefined);
  let eventSource = $state<EventSource | null>(null);
  let sheetOpen = $state(false);
  let lastPlayerStateSaveTime = $state(0);
  let lastCurrentTrackId = $state<number | null>(null);
  let isInitializing = $state(true);

  function initializeAudioService() {
    if (audio) {
      audioService = new AudioService(audio, () => trackStore.nextTrack());
      audioServiceStore.set(audioService);
    }
  }

  function restorePlayerState() {
    if (!audioService) return;

    const {
      current_track_id,
      progress_seconds,
      volume_level,
      is_muted,
      is_shuffle,
      is_repeat,
    } = data.playerState;

    audioService.initializeState(volume_level, is_muted, is_repeat);
    trackStore.setShuffle(is_shuffle);

    if (current_track_id !== null) {
      const trackIndex = data.tracks.findIndex(
        (track: Track) => track.id === current_track_id,
      );
      if (trackIndex >= 0) {
        trackStore.setCurrentTrackIndex(trackIndex);
        audioService.setCurrentTime(progress_seconds);
        audioService.updateAudioSource(data.tracks[trackIndex], false);
        lastCurrentTrackId = current_track_id;
      }
    } else if (data.tracks.length > 0) {
      // If no current track is set but tracks exist, set the first track as current
      trackStore.setCurrentTrackIndex(0);
      audioService.updateAudioSource(data.tracks[0], false);
      lastCurrentTrackId = data.tracks[0].id;
    }
  }

  function setupEventListeners() {
    eventSource = initEventHandlerService();

    if (browser) {
      document.body.addEventListener("toggle-sheet", handleToggleMenu);
      document.addEventListener("visibilitychange", handleVisibilityChange);
    }
  }

  onMount(async () => {
    authConfigStore.initialize();
    trackStore.setTracks(data.tracks);
    initializeAudioService();
    restorePlayerState();
    setupEventListeners();

    // Load swipe functionality on client side
    if (browser) {
      const module = await import("svelte-gestures");
      swipe = (node: HTMLElement) =>
        module.swipe(node, () => ({
          minSwipeDistance: 40,
          timeframe: 300,
          touchAction: "pan-y",
        }));
    }

    isInitializing = false;
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

  // Handle visibility change events
  function handleVisibilityChange() {
    if (document.visibilityState === "hidden") {
      savePlayerState();
    }
  }

  $effect(() => {
    if (
      audioService &&
      $trackStore.currentTrack &&
      $trackStore.currentTrack.id !== lastCurrentTrackId
    ) {
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log(
        "Layout $effect: Track changed - updating audio source. Track ID:",
        $trackStore.currentTrack.id,
        "Previous ID:",
        lastCurrentTrackId,
        "Is initializing:",
        isInitializing,
      );
      const shouldAutoPlay = audioService.isPlaying || !isInitializing;
      audioService.updateAudioSource($trackStore.currentTrack, shouldAutoPlay);
      lastCurrentTrackId = $trackStore.currentTrack.id;
    }
  });

  $effect(() => {
    if ($trackStore.currentTrack && audioService && audioService.isPlaying) {
      // For AI assistant: Debug logging - do not remove this log, user intended to have it
      console.log(
        "Layout $effect: Saving player state. Track ID:",
        $trackStore.currentTrack.id,
        "Is playing:",
        audioService.isPlaying,
        "Last save time:",
        lastPlayerStateSaveTime,
      );
      const now = Date.now();
      if (now - lastPlayerStateSaveTime > 5000) {
        savePlayerState();
        lastPlayerStateSaveTime = now;
      }
    }
  });

  function handleToggleMenu() {
    sheetOpen = !sheetOpen;
  }

  function handleSwipe(event: CustomEvent) {
    if (window.innerWidth < 1000) {
      if (event.detail.direction === "left" && !sheetOpen) {
        sheetOpen = true;
      }
      if (event.detail.direction === "right" && sheetOpen) {
        sheetOpen = false;
      }
    }
  }
</script>

<Sheet.Root bind:open={sheetOpen}>
  <!-- Main content area that uses full viewport scrolling -->
  <main
    class="desktop:pr-64 min-h-screen pr-0 pb-20"
    use:swipe={swipe || undefined}
    onswipe={handleSwipe}
  >
    <div class="desktop:p-4 py-4">
      {@render children()}
    </div>
  </main>

  <Toaster position="top-left" />

  <!-- Desktop Sidebar - positioned fixed on the right -->
  <aside class="desktop:block fixed top-0 right-0 bottom-20 hidden w-64">
    <RightSidebar />
  </aside>

  <!-- Mobile Sidebar Sheet -->
  <Sheet.Content side="right" class="w-64 px-6 py-4">
    <RightSidebar />
  </Sheet.Content>

  <!-- Fixed Player Footer -->
  <PlayerFooter audioService={$audioServiceStore} />

  <audio bind:this={audio} preload="auto" id="mus-audio-element"></audio>
</Sheet.Root>
