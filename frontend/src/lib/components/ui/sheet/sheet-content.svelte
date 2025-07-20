<script lang="ts">
  import { Dialog as SheetPrimitive } from "bits-ui";
  import {
    SheetOverlay,
    SheetPortal,
    type Side,
    sheetVariants,
  } from "./index.js";
  import { cn } from "$lib/utils/ui.js";

  type $$Props = SheetPrimitive.ContentProps & {
    side?: Side;
  };

  let className: $$Props["class"] = undefined;
  export let side: $$Props["side"] = "right";
  export { className as class };
</script>

<SheetPortal>
  <SheetOverlay />
  <SheetPrimitive.Content
    class={cn(sheetVariants({ side }), className)}
    {...$$restProps}
  >
    <slot />
  </SheetPrimitive.Content>
</SheetPortal>
