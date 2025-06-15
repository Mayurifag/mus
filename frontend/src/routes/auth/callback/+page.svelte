<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();

  onMount(() => {
    if (data.token) {
      localStorage.setItem("auth_token", data.token);
    }
    goto("/", { replaceState: true });
  });
</script>

<div class="flex min-h-screen items-center justify-center">
  <div class="text-center">
    <h1 class="mb-2 text-xl font-semibold">Authenticating...</h1>
    <p class="text-muted-foreground">Please wait while we redirect you.</p>
  </div>
</div>
