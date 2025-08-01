<script lang="ts">
  import { trackStore } from "$lib/stores/trackStore";

  import { QrCode, AlertTriangle } from "@lucide/svelte";
  import QRLoginModal from "$lib/components/auth/QRLoginModal.svelte";
  import EffectMonitor from "$lib/components/debug/EffectMonitor.svelte";
  import RecentEvents from "$lib/components/debug/RecentEvents.svelte";
  import ErrorItem from "$lib/components/domain/ErrorItem.svelte";
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

<div class="h-full w-full">
  <!-- QR Login Button - Top Right -->
  <div class="mb-6 flex justify-end">
    <button
      class="icon-glow-effect relative rounded-md p-2 transition-all duration-200"
      onclick={() => (isQrModalOpen = true)}
      title="Open QR code for mobile access"
      aria-label="Open QR code for mobile access"
    >
      <QrCode class="h-8 w-8" />
    </button>
  </div>

  <!-- Errors Section -->
  {#if erroredTracks.length > 0}
    <div class="mb-6 border-t pt-4">
      <h4
        class="text-muted-foreground mb-3 flex items-center gap-2 text-sm font-semibold"
      >
        <AlertTriangle size={16} class="text-red-500" />
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
    <div class="border-t pt-4">
      <h4 class="text-muted-foreground mb-2 text-sm font-semibold">Playback</h4>

      <!-- Timeline View -->
      <div class="space-y-1">
        {#each timelineItems as item (item.position)}
          {#if item.track || item.isCurrent}
            <div class="flex items-center gap-2 text-xs">
              <!-- Pointer -->
              <div class="w-4 text-center">
                {#if item.isCurrent}
                  <span class="text-blue-400">→</span>
                {:else}
                  <span class="text-muted-foreground/30">·</span>
                {/if}
              </div>

              <!-- Track Info -->
              <div
                class="flex-1 {item.isCurrent
                  ? 'font-medium text-blue-400'
                  : 'text-muted-foreground/70'}"
              >
                {#if item.track}
                  {item.track.title} - {formatArtistsForDisplay(
                    item.track.artist,
                  )}
                {:else}
                  <span class="italic">No track</span>
                {/if}
              </div>
            </div>
          {/if}
        {/each}
      </div>
    </div>
  {/if}

  <!-- Recent Events -->
  <RecentEvents />

  <!-- Effect Monitor -->
  <EffectMonitor />

  <!-- QR Login Modal -->
  <QRLoginModal bind:open={isQrModalOpen} />
</div>
