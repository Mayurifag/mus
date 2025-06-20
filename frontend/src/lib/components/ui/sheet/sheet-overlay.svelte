<script lang="ts">
  import { Dialog as SheetPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  type $$Props = SheetPrimitive.OverlayProps;

  let className: $$Props["class"] = undefined;
  export { className as class };
</script>

<SheetPrimitive.Overlay
  class={cn("bg-background/80 fixed inset-0 z-50 backdrop-blur-sm", className)}
  {...$$restProps}
/>
