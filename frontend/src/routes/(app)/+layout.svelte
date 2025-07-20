<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { Snippet } from "svelte";
  import { trackStore } from "$lib/stores/trackStore";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";
  import { permissionsStore } from "$lib/stores/permissionsStore";

  import PlayerFooter from "$lib/components/layout/PlayerFooter.svelte";
  import RightSidebar from "$lib/components/layout/RightSidebar.svelte";
  import { sendPlayerStateBeacon as apiSendPlayerStateBeacon } from "$lib/services/apiClient";
  import { authConfigStore } from "$lib/stores/authConfigStore";
  import { initEventHandlerService } from "$lib/services/eventHandlerService";
  import { AudioService } from "$lib/services/AudioService";
  import { updateEffectStats } from "$lib/utils/monitoredEffect";
  import * as Sheet from "$lib/components/ui/sheet";
  import type { Track } from "$lib/types";
  import { browser } from "$app/environment";
  import { Toaster } from "$lib/components/ui/sonner";
  import type { LayoutData } from "./$types";
  import DropzoneOverlay from "$lib/components/domain/DropzoneOverlay.svelte";
  import TrackMetadataModal from "$lib/components/domain/TrackMetadataModal.svelte";
  import {
    DragDropService,
    type ParsedFileInfo,
  } from "$lib/services/dragDropService";

  // Touch handling for swipe gestures
  let startX = $state<number | null>(null);
  let startY = $state<number | null>(null);
  const SWIPE_THRESHOLD = 50; // Minimum distance for swipe
  const EDGE_THRESHOLD = 30; // Distance from edge to detect edge swipe

  let { data, children }: { data: LayoutData; children: Snippet } = $props();

  let audio: HTMLAudioElement;
  let audioService = $state<AudioService | undefined>(undefined);
  let eventSource = $state<EventSource | null>(null);
  let sheetOpen = $state(false);
  let lastCurrentTrackId: number | null = null;
  let isInitializing = $state(true);

  // Drag and drop state
  let isDraggingFile = $state(false);
  let uploadModalOpen = $state(false);
  let fileToUpload = $state<File | null>(null);
  let parsedFileInfo = $state<ParsedFileInfo | null>(null);
  let dragDropService: DragDropService;

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
        audioService.setTime(progress_seconds);
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

  // Initialize drag and drop service
  function initializeDragDropService() {
    dragDropService = new DragDropService({
      onDragStateChange: (isDragging: boolean) => {
        isDraggingFile = isDragging;
      },
      onFileReady: (fileInfo: ParsedFileInfo) => {
        fileToUpload = fileInfo.file;
        parsedFileInfo = fileInfo;
        uploadModalOpen = true;
      },
    });
  }

  function setupEventListeners() {
    eventSource = initEventHandlerService();

    if (browser) {
      document.body.addEventListener("toggle-sheet", handleToggleMenu);
      document.addEventListener("visibilitychange", handleVisibilityChange);

      // Setup drag and drop service
      initializeDragDropService();
      dragDropService.setupEventListeners();
    }
  }

  onMount(async () => {
    authConfigStore.initialize();
    permissionsStore.set(data.permissions);
    trackStore.setTracks(data.tracks);
    initializeAudioService();
    restorePlayerState();
    setupEventListeners();

    lastCurrentTrackId = data.playerState.current_track_id;
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

      // Remove drag and drop event listeners
      if (dragDropService) {
        dragDropService.removeEventListeners();
      }
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

  // TODO: maybe we should do that inside audio service idk
  $effect(() => {
    updateEffectStats("Layout_TrackChangeHandler");
    if (
      audioService &&
      $trackStore.currentTrack &&
      $trackStore.currentTrack.id !== lastCurrentTrackId
    ) {
      const shouldAutoPlay = audioService.isPlaying || !isInitializing;
      audioService.updateAudioSource($trackStore.currentTrack, shouldAutoPlay);
      lastCurrentTrackId = $trackStore.currentTrack.id;
    }
  });

  $effect(() => {
    updateEffectStats("Layout_PeriodicStateSave");
    if ($trackStore.currentTrack && audioService && audioService.isPlaying) {
      const interval = setInterval(() => {
        savePlayerState();
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
    class="desktop:pr-64 sm700:pb-[calc(var(--footer-height-desktop)+1rem)] min-h-screen overflow-x-hidden pr-0 pb-[calc(var(--footer-height-mobile)+1rem)]"
    style="overscroll-behavior-y: contain; padding-top: var(--safe-area-inset-top);"
    ontouchstart={handleTouchStart}
    ontouchend={handleTouchEnd}
  >
    <div class="desktop:p-4 py-4">
      {@render children()}
    </div>
  </main>

  <Toaster position="top-left" />

  <!-- Desktop Sidebar - positioned fixed on the right -->
  <aside
    class="desktop:block fixed top-0 right-0 hidden w-64"
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

<!-- Drag and drop overlay -->
<DropzoneOverlay visible={isDraggingFile} />

<!-- Upload modal -->
{#if fileToUpload && parsedFileInfo}
  <TrackMetadataModal
    bind:open={uploadModalOpen}
    mode="create"
    file={fileToUpload}
    suggestedTitle={parsedFileInfo.suggestedTitle}
    suggestedArtist={parsedFileInfo.suggestedArtist}
    coverDataUrl={parsedFileInfo.coverInfo?.dataUrl}
    metadata={parsedFileInfo.metadata}
  />
{/if}
