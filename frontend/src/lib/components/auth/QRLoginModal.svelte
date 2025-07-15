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
  const displayUrl = $derived(
    loginUrl ||
      (typeof window !== "undefined"
        ? window.location.origin
        : "https://example.com"),
  );
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay
      class="bg-background/80 fixed inset-0 z-50 backdrop-blur-sm"
    />
    <Dialog.Content
      class="bg-background fixed top-1/2 left-1/2 z-50 w-full max-w-2xl -translate-x-1/2 -translate-y-1/2 rounded-lg border p-6 shadow-lg"
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

      <div class="mt-4">
        <div class="flex flex-col gap-6 sm:flex-row">
          <!-- Left side: QR Code -->
          <div class="flex flex-shrink-0 justify-center sm:justify-start">
            <QRCode data={displayUrl} size={180} />
          </div>

          <!-- Right side: Instructions -->
          <div class="flex-1 space-y-4">
            <div class="space-y-2">
              <p class="text-muted-foreground text-sm">
                {#if loginUrl}
                  Scan the QR code or copy the URL:
                {:else}
                  Scan the QR code to visit this site or copy the URL:
                {/if}
              </p>
              <div class="bg-muted overflow-x-auto rounded p-2">
                <code class="block text-xs whitespace-nowrap select-all"
                  >{displayUrl}</code
                >
              </div>
              {#if !loginUrl}
                <p class="text-muted-foreground text-xs">
                  Note: Authentication is not enabled. This QR code leads to the
                  main site.
                </p>
              {/if}
            </div>

            <!-- PWA Installation Instructions -->
            <div class="space-y-3">
              <h4 class="text-sm font-semibold">Add to Home Screen (PWA)</h4>

              <!-- iOS Instructions -->
              <div class="space-y-1">
                <p class="text-xs font-medium text-blue-400">iOS (Safari):</p>
                <ol
                  class="text-muted-foreground list-inside list-decimal space-y-1 text-xs"
                >
                  <li>Open the link in Safari</li>
                  <li>Tap the Share button (square with arrow)</li>
                  <li>Scroll down and tap "Add to Home Screen"</li>
                  <li>Tap "Add" to confirm</li>
                </ol>
              </div>

              <!-- Android Instructions -->
              <div class="space-y-1">
                <p class="text-xs font-medium text-green-400">
                  Android (Chrome):
                </p>
                <ol
                  class="text-muted-foreground list-inside list-decimal space-y-1 text-xs"
                >
                  <li>Open the link in Chrome</li>
                  <li>Tap the menu (three dots)</li>
                  <li>Tap "Add to Home screen"</li>
                  <li>Tap "Add" to confirm</li>
                </ol>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
