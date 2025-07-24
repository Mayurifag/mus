<script lang="ts">
  import { RotateCcw, Trash2 } from "@lucide/svelte";
  import type { Track } from "$lib/types";
  import { requeueTrack, deleteTrack } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";

  interface Props {
    track: Track;
    onRequeue?: () => void;
    onDelete?: () => void;
  }

  let { track, onRequeue, onDelete }: Props = $props();

  async function handleRequeue() {
    try {
      await requeueTrack(track.id);
      toast.success(`Re-queued "${track.title}" for processing`);
      onRequeue?.();
    } catch (error) {
      toast.error("Failed to re-queue track");
      console.error("Error requeuing track:", error);
    }
  }

  async function handleDelete() {
    if (!confirm(`Are you sure you want to delete "${track.title}"?`)) {
      return;
    }

    try {
      await deleteTrack(track.id);
      toast.success(`Deleted "${track.title}"`);
      onDelete?.();
    } catch (error) {
      toast.error("Failed to delete track");
      console.error("Error deleting track:", error);
    }
  }
</script>

<div
  class="flex items-center justify-between rounded-lg border border-red-200 bg-red-50 p-3"
>
  <div class="min-w-0 flex-1">
    <div class="truncate text-sm font-medium text-red-900">
      {track.title}
    </div>
    <div class="truncate text-xs text-red-700">
      {track.artist}
    </div>
  </div>

  <div class="ml-3 flex items-center gap-2">
    <button
      onclick={handleRequeue}
      class="rounded p-1.5 text-red-600 transition-colors hover:bg-red-100 hover:text-red-800"
      title="Re-queue for processing"
    >
      <RotateCcw size={16} />
    </button>

    <button
      onclick={handleDelete}
      class="rounded p-1.5 text-red-600 transition-colors hover:bg-red-100 hover:text-red-800"
      title="Delete track"
    >
      <Trash2 size={16} />
    </button>
  </div>
</div>
