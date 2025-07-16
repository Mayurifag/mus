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
      class="bg-background fixed top-1/2 left-1/2 z-50 w-full max-w-4xl -translate-x-1/2 -translate-y-1/2 rounded-lg border p-6 shadow-lg"
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
            <QRCode data={displayUrl} size={240} />
          </div>

          <!-- Right side: Instructions -->
          <div class="min-w-0 flex-1 space-y-4">
            <div class="space-y-2">
              <p class="text-muted-foreground text-sm">
                {#if loginUrl}
                  Scan the QR code or copy the URL:
                {:else}
                  Scan the QR code to visit this site on mobile or copy the URL:
                {/if}
              </p>
              <div
                class="bg-muted border-border flex items-center gap-2 rounded border p-2"
              >
                <button
                  class="hover:bg-background flex-shrink-0 cursor-pointer rounded p-1 transition-colors"
                  onclick={() => navigator.clipboard?.writeText(displayUrl)}
                  title="Copy URL to clipboard"
                  aria-label="Copy URL to clipboard"
                >
                  <svg
                    class="h-4 w-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                    />
                  </svg>
                </button>
                <div class="min-w-0 flex-1 overflow-x-auto">
                  <code
                    class="block cursor-text text-xs whitespace-nowrap select-all"
                    >{displayUrl}</code
                  >
                </div>
              </div>
            </div>

            <!-- PWA Installation Instructions -->
            <div class="space-y-3">
              <h4 class="text-sm font-semibold">
                Install as application (PWA)
              </h4>

              <!-- iOS Instructions -->
              <details class="group">
                <summary
                  class="cursor-pointer text-xs font-medium text-blue-400 transition-colors hover:text-blue-300"
                >
                  iOS (Safari)
                </summary>
                <ol
                  class="text-muted-foreground mt-2 list-inside list-decimal space-y-1 text-xs"
                >
                  <li>Open the link in Safari</li>
                  <li>Tap the Share button (square with arrow)</li>
                  <li>Scroll down and tap "Add to Home Screen"</li>
                  <li>Tap "Add" to confirm</li>
                </ol>
              </details>

              <!-- Android Instructions -->
              <details class="group">
                <summary
                  class="cursor-pointer text-xs font-medium text-green-400 transition-colors hover:text-green-300"
                >
                  Android (Chrome)
                </summary>
                <ol
                  class="text-muted-foreground mt-2 list-inside list-decimal space-y-1 text-xs"
                >
                  <li>Open the link in Chrome</li>
                  <li>Tap the menu (three dots)</li>
                  <li>Tap "Add to Home screen"</li>
                  <li>Tap "Add" to confirm</li>
                </ol>
              </details>
            </div>
          </div>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
