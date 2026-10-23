<script lang="ts">
  import { onDestroy } from "svelte";
  import { Slider } from "bits-ui";
  import { Scissors, X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";
  import { fetchTrackCutInfo } from "$lib/services/apiClient";
  import type { AudioCut, Track } from "$lib/types";
  import { formatCutTime, parseCutTime, isValidCut } from "./trackCut";
  import TrackCutPreview from "./TrackCutPreview.svelte";

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
  let times = $state({ start: "", end: "" });
  let confirmed = $state(false);
  let activePreview = $state<"removed" | "join" | null>(null);
  const boundaries = [
    { key: "start", label: "Remove from", placeholder: "0:30.000" },
    { key: "end", label: "To", placeholder: "0:45.000" },
  ] as const;
  let infoRequest: AbortController | undefined;

  const endpoints = $derived({
    start: parseCutTime(times.start),
    end: parseCutTime(times.end),
  });
  const selection = $derived({
    start_seconds: endpoints.start,
    end_seconds: endpoints.end,
  });
  const rangeValid = $derived(isValidCut(selection, duration));
  const timelineSteps = 1000;
  const sliderTimes = $derived(
    rangeValid
      ? [endpoints.start, Math.min(endpoints.end, duration)]
      : [0, duration],
  );
  const sliderValues = $derived(
    sliderTimes.map((seconds) =>
      duration > 0 ? Math.round((seconds / duration) * timelineSteps) : 0,
    ),
  );

  $effect(() => {
    onChange(
      expanded && rangeValid ? selection : null,
      !expanded || (rangeValid && confirmed),
    );
  });

  function resetCut() {
    confirmed = false;
    error = "";
    activePreview = null;
  }

  function nudgeTime(boundary: "start" | "end", delta: number) {
    const value = endpoints[boundary];
    if (!Number.isFinite(value)) return;
    times[boundary] = formatCutTime(
      Math.max(0, Math.min(duration, value + delta)),
    );
    resetCut();
  }

  function changeSlider(values: number[]) {
    if (values[0] === sliderValues[0] && values[1] === sliderValues[1]) return;
    times.start = formatCutTime((values[0] / timelineSteps) * duration);
    times.end = formatCutTime((values[1] / timelineSteps) * duration);
    resetCut();
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
      times.start = formatCutTime(
        Math.max(0, middle - Math.min(5, duration / 4)),
      );
      times.end = formatCutTime(
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
    expanded = false;
    duration = 0;
    resetCut();
  }

  function activatePreview(kind: "removed" | "join") {
    activePreview = kind;
    $audioServiceStore?.pause();
  }

  onDestroy(() => infoRequest?.abort());
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
            max={timelineSteps}
            step={1}
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
                  aria-valuetext={formatCutTime(sliderTimes[index])}
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
          {#each boundaries as boundary (boundary.key)}
            <div class="space-y-1.5">
              <label for={`cut-${boundary.key}`} class="text-sm font-medium"
                >{boundary.label}</label
              >
              <Input
                id={`cut-${boundary.key}`}
                bind:value={times[boundary.key]}
                oninput={resetCut}
                placeholder={boundary.placeholder}
                class="font-mono"
                {disabled}
              />
              <div class="flex gap-2">
                {#each [-1, 1] as delta (delta)}
                  <Button
                    variant="outline"
                    size="sm"
                    class="flex-1"
                    aria-label={`Cut ${boundary.key} ${delta > 0 ? "plus" : "minus"} 1 second`}
                    onclick={() => nudgeTime(boundary.key, delta)}
                    disabled={disabled ||
                      !Number.isFinite(endpoints[boundary.key]) ||
                      (delta < 0
                        ? endpoints[boundary.key] <= 0
                        : endpoints[boundary.key] >= duration)}
                    >{delta > 0 ? "+1 sec" : "−1 sec"}</Button
                  >
                {/each}
              </div>
            </div>
          {/each}
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

        <div class="grid gap-3 sm:grid-cols-2">
          {#key `${track.id}|${times.start}|${times.end}`}
            <TrackCutPreview
              trackId={track.id}
              cut={selection}
              selectionOnly
              active={activePreview === "removed"}
              disabled={disabled || !rangeValid}
              onActivate={() => activatePreview("removed")}
            />
            <TrackCutPreview
              trackId={track.id}
              cut={selection}
              active={activePreview === "join"}
              disabled={disabled || !rangeValid}
              onActivate={() => activatePreview("join")}
            />
          {/key}
        </div>
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
            /><label
              for="confirm-cut"
              class={disabled || !rangeValid
                ? "cursor-not-allowed text-sm leading-none"
                : "cursor-pointer text-sm leading-none"}
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
