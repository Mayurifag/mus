<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    Card,
    CardContent,
    CardHeader,
    CardTitle,
  } from "$lib/components/ui/card";

  interface Props {
    authEnabled: boolean;
    isAuthenticated: boolean;
  }

  let {
    authEnabled,
    isAuthenticated,
    children,
  }: Props & { children: Snippet } = $props();
</script>

{#if authEnabled && !isAuthenticated}
  <div
    class="bg-background/80 fixed inset-0 z-50 flex items-center justify-center backdrop-blur-sm"
  >
    <div class="container mx-auto flex items-center justify-center p-4">
      <Card class="w-full max-w-md">
        <CardHeader class="text-center">
          <CardTitle class="text-2xl font-bold tracking-tight">
            Authentication Required
          </CardTitle>
        </CardHeader>
        <CardContent class="text-center">
          <p class="text-muted-foreground">Navigate to</p>
          <code class="bg-muted mt-2 block rounded px-2 py-1 text-sm"
            >/api/v1/auth/login-by-secret/<span class="text-blue-500"
              >SECRET_KEY</span
            ></code
          >
        </CardContent>
      </Card>
    </div>
  </div>
{:else}
  {@render children()}
{/if}
