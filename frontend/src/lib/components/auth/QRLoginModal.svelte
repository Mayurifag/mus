<script lang="ts">
  import { onMount } from "svelte";
  import { Dialog } from "bits-ui";
  import { X } from "@lucide/svelte";
  import QRCode from "@castlenine/svelte-qrcode";
  import { browser } from "$app/environment";

  interface Props {
    open: boolean;
  }

  let { open = $bindable() }: Props = $props();

  let secretKey = $state<string | null>(null);

  onMount(() => {
    secretKey = localStorage.getItem("auth_token");
  });

  const loginUrl = $derived.by(() => {
    if (!browser || !secretKey) return "";
    const publicApiHost =
      import.meta.env.VITE_PUBLIC_API_HOST || window.location.origin;
    return `${publicApiHost}/api/v1/auth/login-by-secret/${secretKey}`;
  });
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay
      class="bg-background/80 fixed inset-0 z-50 backdrop-blur-sm"
    />
    <Dialog.Content
      class="bg-background fixed top-1/2 left-1/2 z-50 w-full max-w-lg -translate-x-1/2 -translate-y-1/2 rounded-lg border p-6 shadow-lg"
    >
      <div class="flex items-center justify-between">
        <Dialog.Title class="text-lg font-semibold"
          >Login on Mobile</Dialog.Title
        >
        <Dialog.Close
          class="hover:bg-accent hover:text-accent-foreground rounded-sm p-1.5 transition-colors"
        >
          <X class="h-4 w-4" />
        </Dialog.Close>
      </div>

      <div class="mt-4 space-y-4">
        {#if loginUrl}
          <div class="flex justify-center">
            <QRCode data={loginUrl} size={256} />
          </div>
          <div class="space-y-2">
            <p class="text-muted-foreground text-sm">
              Scan the QR code above or copy the URL below:
            </p>
            <div class="bg-muted overflow-x-auto rounded p-2">
              <code class="block text-xs whitespace-nowrap select-all"
                >{loginUrl}</code
              >
            </div>
          </div>
        {:else}
          <div class="text-center">
            <p class="text-muted-foreground">No authentication key available</p>
          </div>
        {/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
