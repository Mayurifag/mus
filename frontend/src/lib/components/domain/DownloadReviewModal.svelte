<script lang="ts">
  import { confirmDownload } from "$lib/services/apiClient";
  import { downloadStore } from "$lib/stores/downloadStore";
  import LazyTrackMetadataModal from "./LazyTrackMetadataModal.svelte";

  let open = $state(false);
  let confirming = false;
  const isAwaitingReview = $derived($downloadStore.state === "awaiting_review");

  $effect(() => {
    if (isAwaitingReview) open = true;
  });

  async function confirm(
    title: string,
    artist: string,
    tags: string[],
    artworkUrl?: string,
  ) {
    const url = $downloadStore.url;
    if (!url) return;
    confirming = true;
    try {
      await confirmDownload(url, title, artist, artworkUrl, tags);
      downloadStore.confirm(url);
      if ($downloadStore.state === "awaiting_review")
        downloadStore.startDownload();
      open = false;
    } catch (error) {
      if (!open && $downloadStore.state === "awaiting_review")
        downloadStore.reset();
      throw error;
    } finally {
      confirming = false;
    }
  }

  function close() {
    if (!confirming && $downloadStore.state === "awaiting_review")
      downloadStore.reset();
    open = false;
  }
</script>

{#if isAwaitingReview || open}
  <LazyTrackMetadataModal
    bind:open
    mode="create"
    isDownload
    suggestedTitle={$downloadStore.title ?? undefined}
    suggestedArtist={$downloadStore.artist ?? undefined}
    suggestedTags={$downloadStore.tags}
    sourceDuration={$downloadStore.duration}
    coverDataUrl={$downloadStore.thumbnailUrl}
    onDownloadConfirm={confirm}
    onClose={close}
  />
{/if}
