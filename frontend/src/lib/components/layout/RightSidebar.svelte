<script lang="ts">
  import { trackStore } from "$lib/stores/trackStore";

  import { QrCode, AlertTriangle, Play } from "@lucide/svelte";
  import QRLoginModal from "$lib/components/auth/QRLoginModal.svelte";
  import EffectMonitor from "$lib/components/debug/EffectMonitor.svelte";
  import RecentEvents from "$lib/components/debug/RecentEvents.svelte";
  import ErrorItem from "$lib/components/domain/ErrorItem.svelte";
  import DownloadManager from "$lib/components/domain/DownloadManager.svelte";
  import type { Track } from "$lib/types";
  import { formatArtistsForDisplay } from "$lib/utils/formatters";
  import { fetchErroredTracks } from "$lib/services/apiClient";
  import { onMount } from "svelte";

  const MIN_HISTORY_FOR_DEBUG = 2;

  let isQrModalOpen = $state(false);
  let erroredTracks = $state<Track[]>([]);

  onMount(async () => {
    erroredTracks = await fetchErroredTracks();
  });

  function removeErroredTrack(trackId: number) {
    erroredTracks = erroredTracks.filter((track) => track.id !== trackId);
  }

  const shouldShowDebugTimeline = $derived(
    $trackStore.is_shuffle &&
      $trackStore.playHistory.length >= MIN_HISTORY_FOR_DEBUG,
  );

  const shouldShowEffectsMonitor =
    import.meta.env.VITE_EFFECTS_DEBUG === "true";

  const timelineItems = $derived.by(() => {
    const items: Array<{
      track: Track | null;
      isCurrent: boolean;
      position: number;
    }> = [];

    if ($trackStore.is_shuffle) {
      const history = $trackStore.playHistory;
      const currentPos = $trackStore.historyPosition;

      // Create timeline from -5 to +5
      for (let offset = -5; offset <= 5; offset++) {
        const historyIndex = currentPos + offset;
        const track =
          historyIndex >= 0 && historyIndex < history.length
            ? history[historyIndex]
            : null;

        items.push({
          track,
          isCurrent: offset === 0,
          position: offset,
        });
      }
    } else {
      // Non-shuffle mode: show sequential tracks
      if (
        $trackStore.currentTrackIndex !== null &&
        $trackStore.tracks.length > 0
      ) {
        const currentIndex = $trackStore.currentTrackIndex;
        const totalTracks = $trackStore.tracks.length;

        for (let offset = -5; offset <= 5; offset++) {
          const trackIndex =
            (currentIndex + offset + totalTracks) % totalTracks;
          const track = $trackStore.tracks[trackIndex];

          items.push({
            track,
            isCurrent: offset === 0,
            position: offset,
          });
        }
      } else {
        // No tracks or no current track
        for (let offset = -5; offset <= 5; offset++) {
          items.push({
            track: null,
            isCurrent: offset === 0,
            position: offset,
          });
        }
      }
    }

    return items;
  });
</script>

<div class="bg-card/50 flex h-full w-full flex-col backdrop-blur-sm">
  <!-- Header with QR Button -->
  <div class="border-border/50 border-b p-4">
    <div class="flex items-center justify-between">
      <h2 class="text-foreground text-lg font-semibold">Controls</h2>
      <button
        class="icon-glow-effect bg-muted/50 hover:bg-muted relative rounded-lg p-2 transition-all duration-200"
        onclick={() => (isQrModalOpen = true)}
        title="Open QR code for mobile access"
        aria-label="Open QR code for mobile access"
      >
        <QrCode class="text-muted-foreground h-5 w-5" />
      </button>
    </div>
  </div>

  <!-- Scrollable Content -->
  <div class="flex-1 overflow-y-auto">
    <!-- Download Manager -->
    <div class="border-border/30 border-b p-4">
      <DownloadManager />
    </div>

    <!-- Errors Section -->
    {#if erroredTracks.length > 0}
      <div class="border-border/30 border-b p-4">
        <h4
          class="text-foreground mb-3 flex items-center gap-2 text-sm font-semibold"
        >
          <AlertTriangle size={16} class="text-destructive" />
          Errors ({erroredTracks.length})
        </h4>

        <div class="space-y-2">
          {#each erroredTracks as track (track.id)}
            <ErrorItem
              {track}
              onRequeue={() => removeErroredTrack(track.id)}
              onDelete={() => removeErroredTrack(track.id)}
            />
          {/each}
        </div>
      </div>
    {/if}

    <!-- Playback Debug Section -->
    {#if shouldShowDebugTimeline}
      <div class="border-border/30 border-b p-4">
        <h4 class="text-foreground mb-3 text-sm font-semibold">
          Playback Timeline
        </h4>

        <!-- Timeline View -->
        <div class="space-y-1.5">
          {#each timelineItems as item (item.position)}
            {#if item.track || item.isCurrent}
              <div
                class="flex items-center gap-3 rounded-md px-2 py-1 text-xs transition-colors {item.isCurrent
                  ? 'bg-accent/10'
                  : 'hover:bg-muted/30'}"
              >
                <!-- Pointer -->
                <div class="w-3 text-center">
                  {#if item.isCurrent}
                    <Play class="text-accent h-3 w-3" />
                  {:else}
                    <span class="text-muted-foreground/40">•</span>
                  {/if}
                </div>

                <!-- Track Info -->
                <div
                  class="flex-1 truncate {item.isCurrent
                    ? 'text-accent font-medium'
                    : 'text-muted-foreground'}"
                >
                  {#if item.track}
                    <span class="block truncate">
                      {item.track.title}
                    </span>
                    <span class="block truncate text-xs opacity-70">
                      {formatArtistsForDisplay(item.track.artist)}
                    </span>
                  {:else}
                    <span class="italic opacity-50">No track</span>
                  {/if}
                </div>
              </div>
            {/if}
          {/each}
        </div>
      </div>
    {/if}

    <!-- Debug Sections -->
    <div class="border-border/30 border-b p-4">
      <RecentEvents />
    </div>

    {#if shouldShowEffectsMonitor}
      <div class="p-4">
        <EffectMonitor />
      </div>
    {/if}
  </div>

  <!-- QR Login Modal -->
  <QRLoginModal bind:open={isQrModalOpen} />
</div>
