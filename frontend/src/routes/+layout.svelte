<script lang="ts">
  let { children } = $props();
  import "../app.css";
  import { Toaster } from "$lib/components/ui/sonner";
</script>

{@render children()}

<!-- <Toaster position="top-right" richColors closeButton /> -->
<Toaster position="top-left" />
