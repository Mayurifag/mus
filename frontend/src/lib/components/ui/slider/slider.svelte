<script lang="ts">
  import { Slider as SliderPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import type { TimeRange } from "$lib/types";

  let {
    class: className,
    value = $bindable([0]),
    max = 100,
    bufferedRanges,
    onValueChange,
    onValueCommit,
    onInput,
    ...restProps
  }: {
    class?: string;
    value?: number[];
    max?: number;
    bufferedRanges?: TimeRange[];
    onValueChange?: (value: number[]) => void;
    onValueCommit?: () => void;
    onInput?: (event: Event) => void;
    [key: string]: unknown;
  } = $props();

  function handleValueChange(newValue: number[]) {
    value = newValue;
    if (onValueChange) {
      onValueChange(newValue);
    }
  }

  function handleValueCommit() {
    if (onValueCommit) {
      onValueCommit();
    }
  }

  function handleInput(event: Event) {
    if (onInput) {
      onInput(event);
    }
  }
</script>

<SliderPrimitive.Root
  bind:value
  {max}
  class={cn(
    "group relative flex w-full touch-none items-center select-none",
    className,
  )}
  onValueChange={handleValueChange}
  on:valuecommit={handleValueCommit}
  on:input={handleInput}
  {...restProps}
  let:thumbs
>
  <span class="bg-muted relative h-2 w-full grow overflow-hidden rounded-full">
    {#if bufferedRanges}
      {#each bufferedRanges as range (range.start + "-" + range.end)}
        <div
          class="bg-accent/20 absolute top-0 h-full"
          style="left: {(range.start / max) * 100}%; width: {((range.end -
            range.start) /
            max) *
            100}%;"
        ></div>
      {/each}
    {/if}
    <SliderPrimitive.Range class="bg-accent absolute h-full" />
  </span>
  {#each thumbs as thumb (thumb)}
    <SliderPrimitive.Thumb
      {thumb}
      class="border-accent/50 bg-accent focus-visible:ring-accent relative z-20 block h-4 w-4 rounded-full border opacity-0 shadow transition-opacity group-hover:opacity-100 focus-visible:ring-1 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
    />
  {/each}
</SliderPrimitive.Root>
