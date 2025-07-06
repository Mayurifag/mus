<script lang="ts">
  import type { TrackHistory } from "$lib/types";
  import { fetchTrackHistory } from "$lib/services/apiClient";
  import { Clock, FileText, Edit, Scan } from "@lucide/svelte";

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

  $effect(() => {
    if (trackId) {
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
  <div class="space-y-3">
    <div class="flex items-center gap-2">
      <Clock class="text-muted-foreground h-4 w-4" />
      <h3 class="text-sm font-medium">5 Last Track Changes</h3>
      <span
        class="bg-muted text-muted-foreground rounded-full px-2 py-0.5 text-xs"
      >
        {history.length}
      </span>
    </div>

    <div class="space-y-2">
      {#each history as entry (entry.id)}
        {@const Icon = getEventIcon(entry.event_type)}
        {@const changes = formatChanges(entry.changes)}

        <div class="bg-muted/30 rounded-lg border p-3">
          <div class="flex items-start gap-3">
            <div class="bg-background rounded-full p-1.5">
              <Icon class="text-muted-foreground h-3 w-3" />
            </div>

            <div class="min-w-0 flex-1 space-y-1">
              <div class="flex items-center justify-between">
                <span class="text-sm font-medium"
                  >{getEventLabel(entry.event_type)}</span
                >
                <span class="text-muted-foreground text-xs">
                  {formatTimestamp(entry.changed_at)}
                </span>
              </div>

              {#if changes.length > 0}
                <div class="space-y-1">
                  {#each changes as change, index (index)}
                    <div class="text-muted-foreground text-xs">{change}</div>
                  {/each}
                </div>
              {:else if entry.event_type === "initial_scan"}
                <div class="text-muted-foreground text-xs">
                  Track discovered: {entry.filename}
                </div>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
