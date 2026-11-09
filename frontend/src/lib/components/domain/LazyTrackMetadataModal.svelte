<script lang="ts">
  import type { ComponentProps } from "svelte";
  import type TrackMetadataModal from "./TrackMetadataModal.svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";

  let {
    open = $bindable(),
    onClose,
    ...props
  }: ComponentProps<typeof TrackMetadataModal> = $props();

  function close() {
    open = false;
    onClose?.();
  }

  function handleOpenChange(newOpen: boolean) {
    if (!newOpen) close();
  }
</script>

{#snippet notice(
  message: string,
  role: "status" | "alert",
  buttonLabel: string,
)}
  <Dialog.Root {open} onOpenChange={handleOpenChange}>
    <Dialog.Content showCloseButton={false}>
      <Dialog.Title>Track editor</Dialog.Title>
      <p {role}>{message}</p>
      <Dialog.Footer>
        <Button variant="outline" onclick={close}>{buttonLabel}</Button>
      </Dialog.Footer>
    </Dialog.Content>
  </Dialog.Root>
{/snippet}

{#await import("./TrackMetadataModal.svelte")}
  {@render notice("Loading track editor...", "status", "Cancel")}
{:then { default: MetadataModal }}
  <MetadataModal bind:open {onClose} {...props} />
{:catch}
  {@render notice("Could not load track editor. Try again.", "alert", "Close")}
{/await}
