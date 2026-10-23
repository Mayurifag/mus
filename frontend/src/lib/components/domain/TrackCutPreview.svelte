<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import { Play } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import { fetchTrackCutPreview } from "$lib/services/apiClient";
  import type { AudioCut } from "$lib/types";

  let {
    trackId,
    cut,
    selectionOnly = false,
    active,
    disabled,
    onActivate,
  }: {
    trackId: string;
    cut: AudioCut;
    selectionOnly?: boolean;
    active: boolean;
    disabled: boolean;
    onActivate: () => void;
  } = $props();

  let url = $state("");
  let loading = $state(false);
  let error = $state("");
  let audio: HTMLAudioElement | undefined = $state();
  let request: AbortController | undefined;

  function clear() {
    request?.abort();
    audio?.pause();
    if (url) URL.revokeObjectURL(url);
    url = "";
    loading = false;
    error = "";
  }

  onDestroy(clear);

  $effect(() => {
    if (!active) audio?.pause();
  });

  async function preview() {
    if (disabled) return;
    clear();
    onActivate();
    const controller = new AbortController();
    request = controller;
    loading = true;
    try {
      const blob = await fetchTrackCutPreview(
        trackId,
        cut,
        selectionOnly,
        controller.signal,
      );
      if (controller.signal.aborted) return;
      url = URL.createObjectURL(blob);
      await tick();
      if (!controller.signal.aborted && active)
        await audio?.play().catch(() => {});
    } catch {
      if (!controller.signal.aborted)
        error = "Preview failed. Please try again.";
    } finally {
      if (!controller.signal.aborted) loading = false;
    }
  }
</script>

<section
  class="bg-muted/40 flex flex-col gap-3 rounded-md p-3"
  aria-label={selectionOnly ? "Removed section" : "Preview join"}
>
  <h4 class="text-sm font-medium">
    {selectionOnly ? "Removed section" : "Preview join"}
  </h4>
  <p class="text-muted-foreground flex-1 text-xs">
    {selectionOnly
      ? "Up to 8 seconds of the selected section"
      : "Up to 4 seconds on either side, with the selection removed"}
  </p>
  <Button
    variant="outline"
    size="sm"
    onclick={preview}
    disabled={disabled || loading}
  >
    <Play class="mr-2 h-4 w-4" />{loading
      ? "Preparing..."
      : selectionOnly
        ? "Hear removed section"
        : "Preview join"}
  </Button>
  {#if url}
    <audio
      bind:this={audio}
      src={url}
      controls
      preload="auto"
      class="h-10 w-full"
      aria-label={selectionOnly
        ? "Selected section preview"
        : "Cut join preview"}
      onplay={onActivate}
      onerror={() => (error = "Unable to play preview. Please try again.")}
    ></audio>
  {:else}
    <p
      class="text-muted-foreground flex h-10 items-center text-xs"
      role="status"
    >
      {loading ? "Preparing audio…" : "Choose a preview to listen."}
    </p>
  {/if}
  {#if error}<p class="text-destructive text-xs" role="alert">{error}</p>{/if}
</section>
