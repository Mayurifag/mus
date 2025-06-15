<script lang="ts">
  import { Button as ButtonPrimitive } from "bits-ui";
  import { buttonVariants } from "./index.js";
  import { cn } from "$lib/utils.js";

  // Instead of trying to use the complex Props type from bits-ui, define just what we need
  let className = "";
  export let variant:
    | "default"
    | "destructive"
    | "outline"
    | "secondary"
    | "ghost"
    | "link" = "default";
  export let size: "default" | "sm" | "lg" | "icon" = "default";
  // Disabling TypeScript errors for this line due to complex types from bits-ui
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export let builders: any[] = [];
  export { className as class };
</script>

<ButtonPrimitive.Root
  {builders}
  class={cn(buttonVariants({ variant, size }), className)}
  type="button"
  {...$$restProps}
  on:click
  on:keydown
>
  <slot />
</ButtonPrimitive.Root>
