<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import { Slider } from "bits-ui";
  import { Scissors, Play, X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";
  import {
    fetchTrackCutInfo,
    fetchTrackCutPreview,
  } from "$lib/services/apiClient";
  import type { AudioCut, Track } from "$lib/types";
  import { formatCutTime, parseCutTime, isValidCut } from "./trackCut";

  let {
    track,
    onChange,
    disabled = false,
  }: {
    track: Track;
    onChange: (cut: AudioCut | null, isValid: boolean) => void;
    disabled?: boolean;
  } = $props();

  let expanded = $state(false);
  let duration = $state(0);
  let loading = $state(false);
  let error = $state("");
  let startText = $state("");
  let endText = $state("");
  let confirmed = $state(false);
  let previewUrl = $state("");
  let previewLoading = $state(false);
  let previewSelection = $state(false);
  let audio: HTMLAudioElement | undefined = $state();
  let infoRequest: AbortController | undefined;
  let previewRequest: AbortController | undefined;

  const selection = $derived({
    start_seconds: parseCutTime(startText),
    end_seconds: parseCutTime(endText),
  });
  const rangeValid = $derived(isValidCut(selection, duration));
  const sliderValues = $derived(
    rangeValid
      ? [selection.start_seconds, Math.min(selection.end_seconds, duration)]
      : [0, duration],
  );

  $effect(() => {
    onChange(
      expanded && rangeValid ? selection : null,
      !expanded || (rangeValid && confirmed),
    );
  });

  function clearPreview() {
    previewRequest?.abort();
    previewRequest = undefined;
    audio?.pause();
    if (previewUrl) URL.revokeObjectURL(previewUrl);
    previewUrl = "";
    previewLoading = false;
  }

  function changeTimes(values?: number[]) {
    if (values) {
      startText = formatCutTime(values[0]);
      endText = formatCutTime(values[1]);
    }
    confirmed = false;
    error = "";
    clearPreview();
  }

  function changeSlider(values: number[]) {
    // Controlled-value updates must not overwrite an unfinished timestamp input.
    if (values[0] === sliderValues[0] && values[1] === sliderValues[1]) return;
    changeTimes(values);
  }

  async function expand() {
    expanded = true;
    loading = true;
    error = "";
    const request = new AbortController();
    infoRequest = request;
    try {
      const info = await fetchTrackCutInfo(track.id, request.signal);
      if (request.signal.aborted) return;
      duration = info.duration_seconds;
      const middle = duration / 2;
      startText = formatCutTime(
        Math.max(0, middle - Math.min(5, duration / 4)),
      );
      endText = formatCutTime(
        Math.min(duration, middle + Math.min(5, duration / 4)),
      );
    } catch {
      if (!request.signal.aborted)
        error = "Could not load audio. Close this section and try again.";
    } finally {
      if (!request.signal.aborted) loading = false;
    }
  }

  function discard() {
    infoRequest?.abort();
    clearPreview();
    expanded = false;
    confirmed = false;
    duration = 0;
    error = "";
  }

  async function preview(selectionOnly: boolean) {
    if (!rangeValid) return;
    clearPreview();
    error = "";
    previewLoading = true;
    previewSelection = selectionOnly;
    const request = new AbortController();
    previewRequest = request;
    try {
      const blob = await fetchTrackCutPreview(
        track.id,
        selection,
        selectionOnly,
        request.signal,
      );
      if (request.signal.aborted) return;
      previewUrl = URL.createObjectURL(blob);
      await tick();
      if (request.signal.aborted) return;
      $audioServiceStore?.pause();
      try {
        await audio?.play();
      } catch {
        // Browsers that block delayed playback still expose the audio controls.
      }
    } catch {
      if (!request.signal.aborted) error = "Preview failed. Please try again.";
    } finally {
      if (!request.signal.aborted) previewLoading = false;
    }
  }

  onDestroy(() => {
    infoRequest?.abort();
    clearPreview();
  });
</script>

<section class="rounded-lg border" aria-label="Remove audio section">
  <div class="flex items-center justify-between gap-3 p-4">
    <div>
      <h3 class="flex items-center gap-2 text-sm font-medium">
        <Scissors class="h-4 w-4" /> Remove section
      </h3>
      <p class="text-muted-foreground mt-1 text-xs">
        Cut an intro, outro, or anything in between.
      </p>
    </div>
    {#if expanded}
      <Button
        variant="ghost"
        size="sm"
        onclick={discard}
        {disabled}
        aria-label="Discard cut"><X class="h-4 w-4" /></Button
      >
    {:else}
      <Button
        variant="outline"
        size="sm"
        onclick={expand}
        {disabled}
        aria-expanded={expanded}>Select section</Button
      >
    {/if}
  </div>

  {#if expanded}
    <div class="space-y-4 border-t p-4">
      {#if loading}
        <p class="text-muted-foreground text-sm" role="status">
          Loading audio duration...
        </p>
      {:else if duration > 0}
        <div class="text-muted-foreground flex justify-between text-xs">
          <span>Drag the handles to mark what to remove</span><span
            class="text-destructive">Selected = removed</span
          >
        </div>
        <div class="px-2">
          <Slider.Root
            type="multiple"
            value={sliderValues}
            onValueChange={changeSlider}
            min={0}
            max={duration}
            step={0.01}
            {disabled}
            class="relative flex h-12 w-full touch-none items-center select-none"
          >
            {#snippet children({ thumbItems })}
              <span
                class="bg-muted relative h-8 w-full overflow-hidden rounded-md"
              >
                <Slider.Range
                  class="bg-destructive/25 border-destructive/50 absolute h-full border-x"
                />
              </span>
              {#each thumbItems as { index } (index)}
                <Slider.Thumb
                  {index}
                  aria-label={index === 0 ? "Cut start" : "Cut end"}
                  aria-valuetext={formatCutTime(sliderValues[index])}
                  class="border-destructive bg-background focus-visible:ring-ring z-10 block h-11 w-4 cursor-ew-resize rounded-md border-2 shadow-sm focus-visible:ring-2 focus-visible:outline-none"
                />
              {/each}
            {/snippet}
          </Slider.Root>
          <div
            class="text-muted-foreground flex justify-between font-mono text-xs"
          >
            <span>0:00</span><span>{formatCutTime(duration)}</span>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <label for="cut-start" class="text-sm font-medium"
              >Remove from</label
            ><Input
              id="cut-start"
              bind:value={startText}
              oninput={() => changeTimes()}
              placeholder="0:30.000"
              class="font-mono"
              {disabled}
            />
          </div>
          <div class="space-y-1.5">
            <label for="cut-end" class="text-sm font-medium">To</label><Input
              id="cut-end"
              bind:value={endText}
              oninput={() => changeTimes()}
              placeholder="0:45.000"
              class="font-mono"
              {disabled}
            />
          </div>
        </div>
        {#if rangeValid}
          <p class="text-sm" aria-live="polite">
            Remove <strong
              >{(selection.end_seconds - selection.start_seconds).toFixed(
                2,
              )}s</strong
            ><span class="text-muted-foreground">
              · New length {formatCutTime(
                duration - (selection.end_seconds - selection.start_seconds),
              )}</span
            >
          </p>
        {:else}
          <p class="text-destructive text-sm" role="alert">
            Enter seconds or m:ss.sss within the track. Keep at least 0.1
            seconds.
          </p>
        {/if}

        <div class="flex flex-wrap gap-2">
          <Button
            variant="outline"
            size="sm"
            onclick={() => preview(false)}
            disabled={disabled || !rangeValid || previewLoading}
            ><Play class="mr-2 h-4 w-4" />{previewLoading && !previewSelection
              ? "Preparing..."
              : "Preview join"}</Button
          >
          <Button
            variant="ghost"
            size="sm"
            onclick={() => preview(true)}
            disabled={disabled || !rangeValid || previewLoading}
            >{previewLoading && previewSelection
              ? "Preparing..."
              : "Hear removed section"}</Button
          >
        </div>
        {#if previewUrl}
          <div class="bg-muted/40 space-y-2 rounded-md p-3">
            <p class="text-muted-foreground text-xs">
              {previewSelection
                ? "First 8 seconds of the selected section"
                : "The actual join · up to 4 seconds on either side, with the selection removed"}
            </p>
            <audio
              bind:this={audio}
              src={previewUrl}
              controls
              preload="auto"
              class="h-10 w-full"
              aria-label={previewSelection
                ? "Selected section preview"
                : "Cut join preview"}
              onplay={() => $audioServiceStore?.pause()}
              onerror={() =>
                (error = "Unable to play preview. Please try again.")}
            ></audio>
          </div>
        {/if}
        <div class="space-y-2 border-t pt-3">
          <p class="text-muted-foreground text-xs">
            Nothing changes until you save. Saving permanently removes this
            section from the original file. Lossy formats are re-encoded; keep a
            backup if needed.
          </p>
          <div class="flex items-start gap-2">
            <Checkbox
              id="confirm-cut"
              bind:checked={confirmed}
              disabled={disabled || !rangeValid}
            /><label for="confirm-cut" class="text-sm leading-none"
              >Apply this cut when saving</label
            >
          </div>
        </div>
      {/if}
      {#if error}<p class="text-destructive text-sm" role="alert">
          {error}
        </p>{/if}
    </div>
  {/if}
</section>
