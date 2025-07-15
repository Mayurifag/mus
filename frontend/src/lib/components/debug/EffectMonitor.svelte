<script lang="ts">
  import {
    effectStats,
    clearEffectStats,
  } from "$lib/stores/effectMonitorStore";
  import { CircleDot, Trash2 } from "@lucide/svelte";

  function formatTimestamp(timestamp: number): string {
    const date = new Date(timestamp);
    const hours = date.getHours().toString().padStart(2, "0");
    const minutes = date.getMinutes().toString().padStart(2, "0");
    const seconds = date.getSeconds().toString().padStart(2, "0");
    return `${hours}:${minutes}:${seconds}`;
  }

  // Sort effects by most recent first
  const sortedEffects = $derived(
    Array.from($effectStats.entries()).sort(
      (a, b) => b[1].lastTriggered - a[1].lastTriggered,
    ),
  );
</script>

{#if $effectStats.size > 0}
  <div class="border-t pt-4">
    <div class="mb-2 flex items-center justify-between">
      <h4 class="text-muted-foreground text-sm font-semibold">
        Effects Monitor
      </h4>
      <button
        onclick={clearEffectStats}
        class="text-muted-foreground/70 hover:text-muted-foreground flex items-center gap-1 text-xs transition-colors"
        title="Clear all effect statistics"
      >
        <Trash2 class="h-3 w-3" />
        Clear
      </button>
    </div>

    <div class="space-y-1">
      {#each sortedEffects as [name, stat] (name)}
        <div class="flex items-center gap-2 text-xs">
          <CircleDot class="h-3 w-3 text-blue-400" />
          <div class="flex-1">
            <div class="font-medium">{name}</div>
            <div class="text-muted-foreground/70">
              Count: {stat.count} | Last: {formatTimestamp(stat.lastTriggered)}
            </div>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
