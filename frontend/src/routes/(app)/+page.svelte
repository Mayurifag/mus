<script lang="ts">
  import { trackStore } from "$lib/stores/trackStore";
  import TrackList from "$lib/components/domain/TrackList.svelte";
  import { Button } from "$lib/components/ui/button";
  import { triggerScan } from "$lib/services/apiClient";
  import { Loader2 } from "lucide-svelte";

  let isScanning = false;
  let scanMessage = "";

  // Access the store with the $ syntax for reactivity
  $: tracks = $trackStore.tracks;
  $: trackCount = tracks.length;

  async function handleScan() {
    if (isScanning) return;

    isScanning = true;
    scanMessage = "";

    try {
      const result = await triggerScan();
      if (result.success) {
        scanMessage = result.message || "Scan completed successfully.";

        // Reload the page to get the updated tracks
        setTimeout(() => {
          window.location.reload();
        }, 2000);
      } else {
        scanMessage = result.message || "Scan failed.";
      }
    } catch (error) {
      scanMessage =
        error instanceof Error
          ? error.message
          : "An error occurred while scanning.";
    } finally {
      isScanning = false;
    }
  }
</script>

<div class="container mx-auto p-4 pb-24">
  <div class="mb-6 flex flex-wrap items-center justify-between gap-4">
    <div>
      <h1 class="mb-1 text-2xl font-bold">Music Library</h1>
      {#if trackCount > 0}
        <p class="text-muted-foreground">
          Found {trackCount} tracks in your library.
        </p>
      {:else}
        <p class="text-muted-foreground">
          No tracks found. Use the scan function to add tracks.
        </p>
      {/if}
    </div>

    <div class="flex items-center gap-4">
      <Button variant="outline" on:click={handleScan} disabled={isScanning}>
        {#if isScanning}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
        {/if}
        Scan Music Library
      </Button>
    </div>
  </div>

  {#if scanMessage}
    <div class="bg-muted mb-4 rounded-md p-3 text-sm">
      {scanMessage}
    </div>
  {/if}

  <TrackList {tracks} />
</div>
