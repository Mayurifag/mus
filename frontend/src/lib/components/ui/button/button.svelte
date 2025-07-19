<script lang="ts">
  import { Button as ButtonPrimitive } from "bits-ui";
  import { buttonVariants } from "./index.js";
  import { cn } from "$lib/utils/ui.js";

  let {
    class: className = "",
    variant = "default",
    size = "default",
    children,
    onclick,
    ...restProps
  }: {
    class?: string;
    variant?:
      | "default"
      | "destructive"
      | "outline"
      | "secondary"
      | "ghost"
      | "link";
    size?: "default" | "sm" | "lg" | "icon";
    children?: import("svelte").Snippet;
    onclick?: (event: MouseEvent) => void;
    [key: string]: unknown;
  } = $props();
</script>

<ButtonPrimitive.Root
  class={cn(buttonVariants({ variant, size }), className)}
  type="button"
  {onclick}
  {...restProps}
>
  {#if children}{@render children()}{/if}
</ButtonPrimitive.Root>
