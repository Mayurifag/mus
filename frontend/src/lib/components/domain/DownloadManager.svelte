<script lang="ts">
  import { downloadStore } from "$lib/stores/downloadStore";
  import { permissionsStore } from "$lib/stores/permissionsStore";
  import { fetchMetadata, confirmDownload } from "$lib/services/apiClient";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import {
    Download,
    Loader2,
    AlertCircle,
    CheckCircle,
    Lock,
  } from "@lucide/svelte";
  import { toast } from "svelte-sonner";
  import TrackMetadataModal from "./TrackMetadataModal.svelte";

  let url = $state("");
  let reviewModalOpen = $state(false);

  async function handleDownload() {
    if (!url.trim()) {
      toast.error("Please enter a URL");
      return;
    }

    const trimmedUrl = url.trim();

    try {
      downloadStore.setFetchingMetadata(trimmedUrl);
      const metadata = await fetchMetadata(trimmedUrl);
      downloadStore.setAwaitingReview({
        title: metadata.title,
        artist: metadata.artist,
        thumbnailUrl: metadata.thumbnail_url,
        duration: metadata.duration,
      });
      // $effect below opens the modal when state becomes awaiting_review
    } catch (error) {
      let errorMessage = "Failed to fetch metadata";

      if (error instanceof Error) {
        if (
          error.message.includes("Too Many Requests") ||
          error.message.includes("429")
        ) {
          errorMessage =
            "Too many download requests. Please wait a moment and try again.";
        } else if (error.message.includes("Download already in progress")) {
          errorMessage =
            "A download is already in progress. Please wait for it to complete.";
        } else if (
          error.message.includes("Service Unavailable") ||
          error.message.includes("503")
        ) {
          errorMessage =
            "Download service is temporarily unavailable. Please try again later.";
        } else {
          errorMessage = error.message;
        }
      }

      downloadStore.setFailed(errorMessage);
      toast.error(errorMessage);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      handleDownload();
    }
  }

  async function handleDownloadConfirm(title: string, artist: string) {
    const storeUrl = $downloadStore.url;
    if (!storeUrl) return;
    await confirmDownload(storeUrl, title, artist);
    url = "";
    downloadStore.startDownload();
    reviewModalOpen = false;
  }

  function handleModalClose() {
    if ($downloadStore.state === "awaiting_review") {
      downloadStore.reset();
    }
    reviewModalOpen = false;
  }

  const isFetchingMetadata = $derived(
    $downloadStore.state === "fetching_metadata",
  );
  const isDownloading = $derived(
    $downloadStore.state === "downloading" ||
      $downloadStore.state === "fetching_metadata",
  );
  const isCompleted = $derived($downloadStore.state === "completed");
  const hasError = $derived($downloadStore.state === "failed");
  const isAwaitingReview = $derived($downloadStore.state === "awaiting_review");
  const canWriteFiles = $derived($permissionsStore.can_write_music_files);
  const progress = $derived($downloadStore.progress);

  $effect(() => {
    if (isAwaitingReview) {
      reviewModalOpen = true;
    }
  });

  $effect(() => {
    if (isCompleted) {
      setTimeout(() => {
        downloadStore.reset();
      }, 2000);
    }
  });
</script>

<div class="space-y-3">
  <h4 class="text-foreground flex items-center gap-2 text-sm font-semibold">
    <Download size={16} class="text-accent" />
    Download from URL
  </h4>

  {#if canWriteFiles}
    <div class="space-y-3">
      <Input
        bind:value={url}
        placeholder="Enter YouTube URL or other supported link"
        disabled={isDownloading || isCompleted}
        onkeydown={handleKeydown}
        class="bg-muted/30 border-border/50 placeholder:text-muted-foreground/60 focus:border-accent/50 focus:ring-accent/20 text-sm"
      />

      <Button
        onclick={handleDownload}
        disabled={isDownloading || isCompleted || !url.trim()}
        size="sm"
        class="w-full font-medium transition-all duration-200 {isDownloading ||
        isCompleted
          ? 'cursor-not-allowed opacity-70'
          : ''} {isCompleted
          ? 'bg-green-600 hover:bg-green-600'
          : 'bg-accent hover:bg-accent/90'} text-accent-foreground"
      >
        {#if isFetchingMetadata}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          Fetching metadata...
        {:else if isDownloading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          Downloading...
        {:else if isCompleted}
          <CheckCircle class="mr-2 h-4 w-4" />
          Completed
        {:else}
          <Download class="mr-2 h-4 w-4" />
          Download
        {/if}
      </Button>

      {#if isDownloading && !isFetchingMetadata && progress !== null}
        <div class="space-y-1">
          <div class="bg-muted/30 h-2 overflow-hidden rounded-full">
            <div
              class="bg-accent h-full rounded-full transition-all duration-300"
              style="width: {progress.percent}%"
            ></div>
          </div>
          <span class="text-muted-foreground text-xs"
            >{progress.percent.toFixed(1)}% · {progress.speed} · ETA {progress.eta}</span
          >
        </div>
      {/if}

      {#if hasError && $downloadStore.error}
        <div
          class="bg-destructive/10 text-destructive flex items-center gap-2 rounded-md p-2 text-sm"
        >
          <AlertCircle size={14} class="flex-shrink-0" />
          <span class="flex-1">{$downloadStore.error}</span>
        </div>
      {/if}
    </div>
  {:else}
    <div
      class="bg-muted/20 text-muted-foreground flex items-center gap-2 rounded-md p-3 text-sm"
    >
      <Lock size={16} class="flex-shrink-0" data-testid="lock-icon" />
      <span class="flex-1">
        Download not available - music directory is read-only
      </span>
    </div>
  {/if}
</div>

{#if isAwaitingReview || reviewModalOpen}
  <TrackMetadataModal
    bind:open={reviewModalOpen}
    mode="create"
    isDownload={true}
    suggestedTitle={$downloadStore.title ?? undefined}
    suggestedArtist={$downloadStore.artist ?? undefined}
    coverDataUrl={$downloadStore.thumbnailUrl}
    onDownloadConfirm={handleDownloadConfirm}
    onClose={handleModalClose}
  />
{/if}
