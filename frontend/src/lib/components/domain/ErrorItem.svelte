<script lang="ts">
  import { RotateCcw, Trash2 } from "@lucide/svelte";
  import type { Track } from "$lib/types";
  import { requeueTrack, deleteTrack } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import ArtistLinks from "$lib/components/domain/ArtistLinks.svelte";

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
  class="border-destructive/30 bg-destructive/10 flex items-center justify-between rounded-lg border p-3"
>
  <div class="min-w-0 flex-1">
    <div class="text-destructive truncate text-sm font-medium">
      {track.title}
    </div>
    <div class="text-destructive/80 truncate text-xs">
      <ArtistLinks artist={track.artist} />
    </div>
  </div>

  <div class="ml-3 flex items-center gap-2">
    <button
      onclick={handleRequeue}
      class="text-destructive hover:bg-destructive/10 rounded p-1.5 transition-colors"
      title="Re-queue for processing"
    >
      <RotateCcw size={16} />
    </button>

    <button
      onclick={handleDelete}
      class="text-destructive hover:bg-destructive/10 rounded p-1.5 transition-colors"
      title="Delete track"
    >
      <Trash2 size={16} />
    </button>
  </div>
</div>
