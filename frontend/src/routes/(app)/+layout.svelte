<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { Snippet } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";
  import { permissionsStore } from "$lib/stores/permissionsStore";
  import type { PermissionsState } from "$lib/stores/permissionsStore";
  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import {
    closeTrackUpdateEvents,
    fetchPermissions,
    fetchPlayerState,
    fetchTracks,
  } from "$lib/services/apiClient";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import { AudioService } from "$lib/services/AudioService";
  import {
    restorePlayerState,
    savePlayerState,
  } from "$lib/services/playerStateService";
  import { updateEffectStats } from "$lib/utils/monitoredEffect";
  import * as Sheet from "$lib/components/ui/sheet";
  import { browser } from "$app/environment";
  import { Toaster } from "$lib/components/ui/sonner";
  import type { PlayerState, Track } from "$lib/types";

  // Touch handling for swipe gestures
  let startX = $state<number | null>(null);
  let startY = $state<number | null>(null);
  const SWIPE_THRESHOLD = 50; // Minimum distance for swipe
  const EDGE_THRESHOLD = 30; // Distance from edge to detect edge swipe

  let { children }: { children: Snippet } = $props();

  let audio: HTMLAudioElement;
  let audioService = $state<AudioService | undefined>(undefined);
  let eventSource = $state<EventSource | null>(null);
  let sheetOpen = $state(false);
  let lastCurrentTrackId: number | null = null;
  let lastPlayRequestId = 0;

  function initializeAudioService() {
    if (audio) {
      audioService = new AudioService(audio);
      audioServiceStore.set(audioService);
    }
  }

  function setupEventListeners() {
    eventSource = initEventHandlerService();

    if (browser) {
      document.body.addEventListener("toggle-sheet", handleToggleMenu);
      document.addEventListener("visibilitychange", handleVisibilityChange);
    }
  }

  onMount(() => {
    Promise.all([fetchTracks(), fetchPlayerState(), fetchPermissions()])
      .then(
        ([tracks, playerState, permissions]: [
          Track[],
          PlayerState,
          PermissionsState,
        ]) => {
          permissionsStore.set(permissions);
          trackStore.setTracks(tracks);
          initializeAudioService();
          lastCurrentTrackId = restorePlayerState(
            audioService,
            tracks,
            playerState,
          );
          setupEventListeners();
        },
      )
      .catch((error) => {
        console.error("Failed to load initial app data", error);
        permissionsStore.set({ can_write_music_files: false });
        trackStore.setTracks([]);
        initializeAudioService();
        setupEventListeners();
      });
  });

  // Clean up event source on component destroy
  onDestroy(() => {
    if (eventSource) {
      closeTrackUpdateEvents(eventSource);
    }

    // Clean up AudioService
    if (audioService) {
      audioService.destroy();
      audioServiceStore.set(undefined);
    }

    // Remove event listeners - only in browser
    if (browser && document) {
      document.body.removeEventListener("toggle-sheet", handleToggleMenu);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    }
  });

  // Handle visibility change events
  function handleVisibilityChange() {
    if (document.visibilityState === "hidden") {
      savePlayerState(
        audioService,
        $trackStore.currentTrack,
        $trackStore.is_shuffle,
      );
    }
  }

  // TODO: maybe we should do that inside audio service idk
  $effect(() => {
    updateEffectStats("Layout_TrackChangeHandler");
    if (
      audioService &&
      $trackStore.currentTrack &&
      $trackStore.currentTrack.id !== lastCurrentTrackId
    ) {
      const shouldAutoPlay =
        audioService.isPlaying ||
        $trackStore.playRequestId !== lastPlayRequestId;
      audioService.updateAudioSource($trackStore.currentTrack, shouldAutoPlay);
      lastCurrentTrackId = $trackStore.currentTrack.id;
      lastPlayRequestId = $trackStore.playRequestId;
    }
  });

  $effect(() => {
    updateEffectStats("Layout_PeriodicStateSave");
    if ($trackStore.currentTrack && audioService && audioService.isPlaying) {
      const interval = setInterval(() => {
        savePlayerState(
          audioService,
          $trackStore.currentTrack,
          $trackStore.is_shuffle,
        );
      }, 5000);
      return () => clearInterval(interval);
    }
  });

  function handleToggleMenu() {
    sheetOpen = !sheetOpen;
  }

  function handleTouchStart(event: TouchEvent) {
    if (window.innerWidth >= 1000) return;
    startX = event.touches[0].clientX;
    startY = event.touches[0].clientY;
  }

  function handleTouchEnd(event: TouchEvent) {
    if (window.innerWidth >= 1000 || !startX || !startY) return;

    const deltaX = event.changedTouches[0].clientX - startX;
    const deltaY = Math.abs(event.changedTouches[0].clientY - startY);

    const cleanup = () => {
      startX = null;
      startY = null;
    };

    if (Math.abs(deltaX) < SWIPE_THRESHOLD || deltaY > Math.abs(deltaX)) {
      return cleanup();
    }

    // Open sidebar: swipe from right edge to left when closed
    if (
      deltaX < -SWIPE_THRESHOLD &&
      startX > window.innerWidth - EDGE_THRESHOLD &&
      !sheetOpen
    ) {
      sheetOpen = true;
    }

    cleanup();
  }
</script>

<Sheet.Root bind:open={sheetOpen}>
  <!-- Main content area that uses full viewport scrolling -->
  <main
    class="desktop:pr-64 sm700:pb-[calc(var(--footer-height-desktop)+1rem)] min-h-screen overflow-x-clip pr-0 pb-[calc(var(--footer-height-mobile)+1rem)]"
    style="overscroll-behavior-y: contain; padding-top: var(--safe-area-inset-top);"
    ontouchstart={handleTouchStart}
    ontouchend={handleTouchEnd}
  >
    <div class="desktop:p-4 py-4">
      {@render children()}
    </div>
  </main>

  <Toaster position="top-right" />

  <!-- Desktop Sidebar - positioned fixed on the right -->
  <aside
    class="desktop:block fixed top-0 right-0 bottom-[var(--footer-height-desktop)] hidden w-64"
    style="padding-top: var(--safe-area-inset-top); padding-right: var(--safe-area-inset-right);"
  >
    <RightSidebar />
  </aside>

  <!-- Mobile Sidebar Sheet -->
  <Sheet.Content side="right" class="w-64 px-6 py-4">
    <RightSidebar />
  </Sheet.Content>

  <!-- Fixed Player Footer -->
  <PlayerFooter audioService={$audioServiceStore} />

  <audio bind:this={audio} preload="auto" id="mus-audio-element" class="hidden"
  ></audio>
</Sheet.Root>
