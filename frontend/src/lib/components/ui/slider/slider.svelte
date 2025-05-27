<script lang="ts">
  import { Slider as SliderPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  type $$Props = SliderPrimitive.Props;

  let className: $$Props["class"] = undefined;
  export let value: $$Props["value"] = [0];
  export { className as class };
</script>

<SliderPrimitive.Root
  bind:value
  class={cn(
    "relative flex w-full touch-none items-center select-none",
    className,
  )}
  {...$$restProps}
  let:thumbs
>
  <span
    class="bg-muted relative h-1.5 w-full grow overflow-hidden rounded-full"
  >
    <SliderPrimitive.Range class="bg-accent absolute h-full" />
  </span>
  {#each thumbs as thumb (thumb)}
    <SliderPrimitive.Thumb
      {thumb}
      class="border-accent/50 bg-background focus-visible:ring-accent block h-4 w-4 rounded-full border shadow transition-colors focus-visible:ring-1 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
    />
  {/each}
</SliderPrimitive.Root>
