<script lang="ts">
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Save, Trash2 } from "@lucide/svelte";

  let {
    mode,
    canDelete,
    isUploading,
    isFormValid,
    hasSavableChanges,
    onCancel,
    onSave,
    onDeleteClick,
  }: {
    mode: "edit" | "create";
    canDelete: boolean;
    isUploading: boolean;
    isFormValid: boolean;
    hasSavableChanges: boolean;
    onCancel: () => void;
    onSave: () => void;
    onDeleteClick: () => void;
  } = $props();

  const saveTitle = $derived(
    !isFormValid
      ? "Please fill in required fields (title and primary artist)"
      : !hasSavableChanges
        ? "No changes to save"
        : "Save",
  );
</script>

<Dialog.Footer class="!justify-between">
  <div class="mr-auto">
    {#if mode === "edit" && canDelete}
      <Button
        variant="outline"
        size="sm"
        onclick={onDeleteClick}
        disabled={isUploading}
        title="Delete track"
        class="border-destructive/20 text-destructive hover:bg-destructive hover:text-destructive-foreground"
      >
        <Trash2 class="mr-2 h-4 w-4" />
        Delete
      </Button>
    {/if}
  </div>

  <div class="flex gap-2">
    <Button
      variant="outline"
      onclick={onCancel}
      class="cursor-pointer"
      disabled={isUploading}
    >
      Cancel
    </Button>
    <Button
      onclick={onSave}
      disabled={!hasSavableChanges || isUploading}
      class="cursor-pointer"
      title={saveTitle}
    >
      {#if isUploading}
        Saving...
      {:else}
        <Save class="mr-2 h-4 w-4" />
        Save
      {/if}
    </Button>
  </div>
</Dialog.Footer>
