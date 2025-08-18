<script lang="ts">
  import { downloadStore } from "$lib/stores/downloadStore";
  import { permissionsStore } from "$lib/stores/permissionsStore";
  import { startDownload } from "$lib/services/apiClient";
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

  let url = $state("");

  async function handleDownload() {
    if (!url.trim()) {
      toast.error("Please enter a URL");
      return;
    }

    try {
      downloadStore.startDownload();
      await startDownload(url.trim());
      url = "";
    } catch (error) {
      let errorMessage = "Download failed";

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

  const isDownloading = $derived($downloadStore.state === "downloading");
  const isCompleted = $derived($downloadStore.state === "completed");
  const hasError = $derived($downloadStore.state === "failed");
  const canWriteFiles = $derived($permissionsStore.can_write_music_files);

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
        disabled={isDownloading}
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
        {#if isDownloading}
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
