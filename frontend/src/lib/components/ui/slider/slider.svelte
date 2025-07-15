<script lang="ts">
  import { Slider as SliderPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import type { TimeRange } from "$lib/types";

  let {
    class: className,
    value = $bindable(0),
    max = 100,
    bufferedRanges,
    onValueChange,
    onValueCommit,
    onpointerdown,
    ...restProps
  }: {
    class?: string;
    value?: number;
    max?: number;
    bufferedRanges?: TimeRange[];
    onValueChange?: (value: number[]) => void;
    onValueCommit?: (value: number) => void;
    onpointerdown?: (event: PointerEvent) => void;
    [key: string]: unknown;
  } = $props();
</script>

<SliderPrimitive.Root
  type="single"
  bind:value
  onValueChange={(v: number) => onValueChange?.([v])}
  {onValueCommit}
  {max}
  class={cn(
    "group relative flex w-full touch-none items-center select-none",
    className,
  )}
  {onpointerdown}
  {...restProps}
>
  {#snippet children({ thumbItems })}
    <span
      class="bg-muted relative h-2 w-full grow overflow-hidden rounded-full"
    >
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
      <SliderPrimitive.Range
        class="bg-accent absolute h-full"
        style="left: 0; width: {(value / max) * 100}%;"
      />
    </span>
    {#each thumbItems as { index } (index)}
      <SliderPrimitive.Thumb
        {index}
        class="border-accent/50 bg-accent focus-visible:ring-accent relative z-20 block h-4 w-4 rounded-full border opacity-0 shadow transition-opacity group-hover:opacity-100 focus-visible:ring-1 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
      />
    {/each}
  {/snippet}
</SliderPrimitive.Root>
