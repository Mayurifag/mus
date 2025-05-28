<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { toast } from "svelte-sonner";
  import { triggerTestToasts } from "$lib/services/apiClient";

  async function handleTestToasts() {
    try {
      const result = await triggerTestToasts();
      console.log("Test toasts triggered:", result);
    } catch (error) {
      console.error("Error triggering test toasts:", error);
      toast.error("Failed to trigger test toasts");
    }
  }
</script>

<div class="h-full w-full p-4">
  <!-- Toast Test Section -->
  <div class="bg-muted/50 mb-6 rounded-lg border p-4">
    <h3 class="mb-2 text-sm font-semibold">Toast Test</h3>
    <p class="text-muted-foreground mb-3 text-xs">
      Test toast notifications. You should see 5 different toasts appear in the
      top-right corner.
    </p>
    <Button
      on:click={handleTestToasts}
      variant="outline"
      size="sm"
      class="w-full"
    >
      Test Toasts
    </Button>
  </div>
</div>
