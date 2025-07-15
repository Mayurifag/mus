<script lang="ts">
  import { recentEventsStore } from "$lib/stores/recentEventsStore";
  import {
    FileText,
    FilePlus,
    FileX,
    FileEdit,
    Info,
    CheckCircle,
    AlertCircle,
    XCircle,
  } from "@lucide/svelte";

  function formatTimestamp(timestamp: number): string {
    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, "0");
    const minutes = date.getMinutes().toString().padStart(2, "0");
    const seconds = date.getSeconds().toString().padStart(2, "0");
    return `${hours}:${minutes}:${seconds}`;
  }

  function getEventIcon(actionKey: string | null) {
    switch (actionKey) {
      case "track_added":
        return FilePlus;
      case "track_updated":
        return FileEdit;
      case "track_deleted":
        return FileX;
      default:
        return FileText;
    }
  }

  function getLevelIcon(level: string | null) {
    switch (level) {
      case "success":
        return CheckCircle;
      case "error":
        return XCircle;
      case "warning":
        return AlertCircle;
      default:
        return Info;
    }
  }

  function getLevelColor(level: string | null): string {
    switch (level) {
      case "success":
        return "text-green-400";
      case "error":
        return "text-red-400";
      case "warning":
        return "text-yellow-400";
      default:
        return "text-blue-400";
    }
  }
</script>

{#if $recentEventsStore.length > 0}
  <div class="border-t pt-4">
    <h4 class="text-muted-foreground mb-2 text-sm font-semibold">
      Recent Events
    </h4>
    <div class="space-y-2">
      {#each $recentEventsStore as event (event.id)}
        <div class="flex items-start gap-2 text-xs">
          <!-- Time -->
          <div class="text-muted-foreground/50 w-12 flex-shrink-0 text-right">
            {formatTimestamp(event.timestamp)}
          </div>

          <!-- Icon -->
          <div class="mt-0.5 flex-shrink-0">
            {#if event.action_key}
              {@const IconComponent = getEventIcon(event.action_key)}
              <IconComponent
                class="h-3 w-3 {getLevelColor(event.message_level)}"
              />
            {:else}
              {@const IconComponent = getLevelIcon(event.message_level)}
              <IconComponent
                class="h-3 w-3 {getLevelColor(event.message_level)}"
              />
            {/if}
          </div>

          <!-- Message -->
          <div class="min-w-0 flex-1">
            {#if event.message_to_show}
              <div class="text-muted-foreground/80 break-words">
                {event.message_to_show}
              </div>
            {:else if event.action_key}
              <div class="text-muted-foreground/60 italic">
                {event.action_key}
              </div>
            {:else}
              <div class="text-muted-foreground/40 italic">Unknown event</div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
