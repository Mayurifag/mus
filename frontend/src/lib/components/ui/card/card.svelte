<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";
  import { cn } from "$lib/utils/ui.js";

  type $$Props = HTMLAttributes<HTMLDivElement>;

  let className: $$Props["class"] = undefined;
  export { className as class };
</script>

<div
  class={cn("bg-card text-card-foreground rounded-xl border shadow", className)}
  {...$$restProps}
  on:click
  on:focusin
  on:focusout
  on:mouseenter
  on:mouseleave
>
  <slot />
</div>
