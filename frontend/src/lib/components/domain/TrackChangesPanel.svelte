<script lang="ts">
  import type { TrackHistory } from "$lib/types";
  import { fetchTrackHistory } from "$lib/services/apiClient";
  import { Clock, FileText, Edit, Scan } from "@lucide/svelte";
  import { formatArtistsForDisplay } from "$lib/utils";

  let {
    trackId,
    hasChanges = $bindable(),
    changesCount = $bindable(),
  }: {
    trackId: number;
    hasChanges?: boolean;
    changesCount?: number;
  } = $props();

  let history = $state<TrackHistory[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function loadHistory() {
    try {
      loading = true;
      error = null;
      history = await fetchTrackHistory(trackId);
    } catch (err) {
      error = "Failed to load track history";
      console.error("Error loading track history:", err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    hasChanges = history.length > 0;
    changesCount = history.length;
  });

  let currentTrackId = $state<number | null>(null);

  $effect(() => {
    if (trackId && trackId !== currentTrackId) {
      currentTrackId = trackId;
      loadHistory();
    }
  });

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  function getEventIcon(eventType: string) {
    switch (eventType) {
      case "initial_scan":
        return Scan;
      case "edit":
        return Edit;
      case "metadata_update":
        return FileText;
      default:
        return Clock;
    }
  }

  function getEventLabel(eventType: string): string {
    switch (eventType) {
      case "initial_scan":
        return "Initial scan";
      case "edit":
        return "Manual edit";
      case "metadata_update":
        return "Metadata update";
      default:
        return "Unknown";
    }
  }

  function formatChanges(
    changes: Record<string, unknown> | null | undefined,
  ): string[] {
    if (!changes) return [];

    const changesList: string[] = [];

    if (
      changes.title &&
      typeof changes.title === "object" &&
      changes.title !== null &&
      "old" in changes.title &&
      "new" in changes.title
    ) {
      changesList.push(
        `Title: "${changes.title.old}" → "${changes.title.new}"`,
      );
    }

    if (
      changes.artist &&
      typeof changes.artist === "object" &&
      changes.artist !== null &&
      "old" in changes.artist &&
      "new" in changes.artist
    ) {
      changesList.push(
        `Artist: "${changes.artist.old}" → "${changes.artist.new}"`,
      );
    }

    if (changes.file_renamed) {
      changesList.push("File was renamed");
    }

    return changesList;
  }
</script>

{#if loading}
  <div class="bg-muted/30 rounded-lg border p-4">
    <div class="flex items-center gap-2">
      <Clock class="h-4 w-4 animate-spin" />
      <span class="text-muted-foreground text-sm">Loading track changes...</span
      >
    </div>
  </div>
{:else if error}
  <div class="border-destructive/20 bg-destructive/5 rounded-lg border p-4">
    <div class="flex items-center gap-2">
      <Clock class="text-destructive h-4 w-4" />
      <span class="text-destructive text-sm">{error}</span>
    </div>
  </div>
{:else if history.length > 0}
  <div class="space-y-4">
    <div class="flex items-center gap-2">
      <Clock class="text-muted-foreground h-4 w-4" />
      <h3 class="text-sm font-medium">Track Changes</h3>
      <span
        class="bg-muted text-muted-foreground rounded-full px-2 py-0.5 text-xs"
      >
        {history.length}
      </span>
    </div>

    <!-- Comprehensive Debug Table -->
    <div class="overflow-x-auto">
      <table class="w-full border-collapse text-xs">
        <thead>
          <tr class="border-muted border-b">
            <th class="text-muted-foreground p-2 text-left font-medium">ID</th>
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Track ID</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Event</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Title</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Artist</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Duration</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Filename</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Changed At</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Changes</th
            >
            <th class="text-muted-foreground p-2 text-left font-medium"
              >Full Snapshot</th
            >
          </tr>
        </thead>
        <tbody>
          {#each history as entry (entry.id)}
            {@const changes = formatChanges(entry.changes)}
            {@const Icon = getEventIcon(entry.event_type)}
            <tr class="border-muted/50 hover:bg-muted/20 border-b">
              <td class="text-muted-foreground p-2">{entry.id}</td>
              <td class="text-muted-foreground p-2">{entry.track_id}</td>
              <td class="p-2">
                <span class="inline-flex items-center gap-1">
                  <Icon class="h-3 w-3" />
                  <span class="font-medium"
                    >{getEventLabel(entry.event_type)}</span
                  >
                </span>
              </td>
              <td class="max-w-32 truncate p-2" title={entry.title}
                >{entry.title}</td
              >
              <td class="max-w-32 truncate p-2" title={entry.artist}
                >{formatArtistsForDisplay(entry.artist)}</td
              >
              <td class="text-muted-foreground p-2">{entry.duration}s</td>
              <td
                class="text-muted-foreground max-w-40 truncate p-2"
                title={entry.filename}>{entry.filename}</td
              >
              <td class="text-muted-foreground p-2"
                >{formatTimestamp(entry.changed_at)}</td
              >
              <td class="max-w-48 p-2">
                {#if changes.length > 0}
                  <div class="space-y-1">
                    {#each changes as change, index (index)}
                      <div class="text-muted-foreground">{change}</div>
                    {/each}
                  </div>
                {:else if entry.event_type === "initial_scan"}
                  <span class="text-muted-foreground italic"
                    >Track discovered</span
                  >
                {:else}
                  <span class="text-muted-foreground italic">No changes</span>
                {/if}
              </td>
              <td class="max-w-48 p-2">
                {#if entry.full_snapshot}
                  <details class="text-muted-foreground">
                    <summary class="hover:text-foreground cursor-pointer"
                      >View snapshot</summary
                    >
                    <pre
                      class="bg-muted/30 mt-1 overflow-x-auto rounded p-2 text-xs">{JSON.stringify(
                        entry.full_snapshot,
                        null,
                        2,
                      )}</pre>
                  </details>
                {:else}
                  <span class="text-muted-foreground italic">No snapshot</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
{/if}
