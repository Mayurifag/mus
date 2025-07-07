<script lang="ts">
  import { cn } from "$lib/utils.js";
  import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";

  let {
    ref = $bindable(null),
    class: className,
    inset,
    variant = "default",
    ...restProps
  }: DropdownMenuPrimitive.ItemProps & {
    inset?: boolean;
    variant?: "default" | "destructive";
  } = $props();
</script>

<DropdownMenuPrimitive.Item
  bind:ref
  data-slot="dropdown-menu-item"
  data-inset={inset}
  data-variant={variant}
  class={cn(
    "data-highlighted:text-accent-foreground data-[variant=destructive]:text-destructive data-[variant=destructive]:data-highlighted:bg-destructive/10 dark:data-[variant=destructive]:data-highlighted:bg-destructive/20 data-[variant=destructive]:data-highlighted:text-destructive data-[variant=destructive]:*:[svg]:!text-destructive [&_svg:not([class*='text-'])]:text-muted-foreground relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[inset]:pl-8 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
    className,
  )}
  {...restProps}
/>

<style>
  :global(
    [data-slot="dropdown-menu-item"][data-highlighted]:not(
        [data-variant="destructive"]
      )
  ) {
    color: white;
    text-shadow:
      0 0 8px hsl(var(--accent) / 0.8),
      0 0 4px hsl(var(--accent) / 1);
  }

  :global(
    [data-slot="dropdown-menu-item"][data-highlighted]:not(
        [data-variant="destructive"]
      )
      svg
  ) {
    color: white;
    filter: drop-shadow(0 0 8px hsl(var(--accent) / 0.8))
      drop-shadow(0 0 4px hsl(var(--accent) / 1));
  }
</style>
