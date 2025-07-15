<script lang="ts">
  import { Dialog } from "bits-ui";
  import { X } from "@lucide/svelte";
  import QRCode from "@castlenine/svelte-qrcode";
  import { authConfigStore } from "$lib/stores/authConfigStore";

  interface Props {
    open: boolean;
  }

  let { open = $bindable() }: Props = $props();

  const loginUrl = $derived($authConfigStore.magicLinkUrl);
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
          class="hover:bg-accent hover:text-accent-foreground cursor-pointer rounded-sm p-1.5 transition-colors"
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
              Scan the QR code or copy the URL:
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
