<script lang="ts">
  import type { Track } from "$lib/types";
  import { onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { Button } from "$lib/components/ui/button";
  import { updateTrack } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import { Plus, X } from "@lucide/svelte";

  let {
    open = $bindable(),
    track,
  }: {
    open: boolean;
    track: Track;
  } = $props();

  let formState = $state({
    title: "",
    renameFile: true,
  });

  let artistIdCounter = 0;
  let artists = $state<{ id: number; value: string }[]>([]);

  function resetState() {
    formState.title = track.title;
    formState.renameFile = true;
    artists = (track.artist?.split(";") ?? [])
      .map((a) => a.trim())
      .filter(Boolean)
      .map((value) => ({ id: artistIdCounter++, value }));
  }

  onMount(() => {
    resetState();
  });

  $effect(() => {
    if (open) {
      resetState();
    }
  });

  const currentArtistString = $derived(
    artists.map((a) => a.value.trim()).join(";"),
  );

  const originalFilename = $derived(track.file_path.split("/").pop() ?? "");
  const fileExtension = $derived(originalFilename.split(".").pop() ?? "");

  function generateSanitizedFilename(
    title: string,
    artistsArray: { value: string }[],
    extension: string,
  ): string {
    const sanitize = (name: string) => name.replace(/[<>:"/\\|?*]/g, "");
    const artistsString = artistsArray
      .map((a) => a.value)
      .filter((a) => a.trim())
      .join(", ");
    const sanitizedArtists = sanitize(artistsString);
    const sanitizedTitle = sanitize(title);

    if (!sanitizedArtists || !sanitizedTitle) {
      return "";
    }

    return `${sanitizedArtists} - ${sanitizedTitle}.${extension}`;
  }

  const newFilenamePreview = $derived(
    generateSanitizedFilename(formState.title, artists, fileExtension),
  );

  const changes = $derived.by(() => {
    const metadataChanged =
      formState.title !== track.title || currentArtistString !== track.artist;
    const filenameWouldChange =
      newFilenamePreview && newFilenamePreview !== originalFilename;

    const hasSavableChanges =
      metadataChanged || (formState.renameFile && filenameWouldChange);

    const hasUnsavedChanges = metadataChanged || formState.renameFile !== true;

    return {
      hasSavableChanges,
      hasUnsavedChanges,
      showRenameSection: metadataChanged || filenameWouldChange,
    };
  });

  function addArtist() {
    artists = [...artists, { id: artistIdCounter++, value: "" }];
  }

  function removeArtist(id: number) {
    artists = artists.filter((artist) => artist.id !== id);
  }

  async function handleSave() {
    const payload: { title?: string; artist?: string; rename_file?: boolean } =
      {};

    if (formState.title !== track.title) {
      payload.title = formState.title;
    }
    if (currentArtistString !== track.artist) {
      payload.artist = currentArtistString;
    }
    if (formState.renameFile) {
      payload.rename_file = true;
    }

    try {
      await updateTrack(track.id, payload);
      toast.success("Track updated successfully");
      open = false;

      // Trigger a page reload to refresh the track list
      window.location.reload();
    } catch (error) {
      console.error("Error updating track:", error);
      toast.error("Failed to update track");
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="max-h-[90vh] overflow-y-auto sm:max-w-[600px]">
    <Dialog.Header>
      <Dialog.Title>Edit Track</Dialog.Title>
      <Dialog.Description>
        Make changes to the track metadata. Click save when you're done.
      </Dialog.Description>
    </Dialog.Header>

    <div class="grid gap-6 py-6">
      <div class="flex items-center gap-6">
        <div
          class="bg-muted h-20 w-20 flex-shrink-0 overflow-hidden rounded-lg"
        >
          {#if track.has_cover && track.cover_original_url}
            <img
              src={track.cover_original_url}
              alt="Track cover"
              class="h-full w-full object-cover"
            />
          {:else}
            <img
              src="/images/no-cover.svg"
              alt="No cover"
              class="h-full w-full object-cover"
            />
          {/if}
        </div>
        <div class="min-w-0 flex-1">
          <p class="truncate text-lg font-semibold">{track.title}</p>
          <p class="text-muted-foreground truncate">{track.artist}</p>
          <p class="text-muted-foreground mt-1 truncate text-sm">
            {originalFilename}
          </p>
        </div>
      </div>

      <div class="space-y-4">
        <div class="space-y-2">
          <label for="title" class="text-sm font-medium">Title</label>
          <Input id="title" bind:value={formState.title} class="w-full" />
        </div>

        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-sm font-medium">Artists</span>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onclick={addArtist}
              class="h-8 cursor-pointer px-3"
            >
              <Plus class="mr-1 h-4 w-4" />
              Add Artist
            </Button>
          </div>

          <div class="space-y-2">
            {#each artists as artist, i (artist.id)}
              <div class="flex items-center gap-2">
                <Input
                  bind:value={artist.value}
                  placeholder={i === 0 ? "Primary artist" : "Additional artist"}
                  class="flex-1"
                />
                {#if artists.length > 1}
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    onclick={() => removeArtist(artist.id)}
                    class="text-muted-foreground hover:text-destructive h-10 w-10 cursor-pointer p-0"
                  >
                    <X class="h-4 w-4" />
                  </Button>
                {/if}
              </div>
            {/each}
          </div>

          <p class="text-muted-foreground text-xs">
            Add multiple artists for collaborations and featured artists.
          </p>
        </div>
      </div>

      {#if changes.showRenameSection}
        <div class="mt-6 border-t pt-6">
          <div class="space-y-4">
            <div class="flex items-center space-x-3">
              <Checkbox
                id="rename"
                bind:checked={formState.renameFile}
                class="cursor-pointer"
              />
              <label for="rename" class="cursor-pointer text-base font-medium">
                Rename file to match metadata
              </label>
            </div>

            {#if formState.renameFile}
              <div
                class="space-y-3 rounded-lg bg-slate-50 p-4 dark:bg-slate-900"
              >
                <div class="flex items-center gap-2">
                  <span
                    class="text-sm font-medium text-slate-600 dark:text-slate-400"
                  >
                    New filename:
                  </span>
                </div>
                <div class="rounded-md border bg-white p-3 dark:bg-slate-800">
                  {#if newFilenamePreview}
                    <code
                      class="break-all text-sm text-slate-800 dark:text-slate-200"
                    >
                      {newFilenamePreview}
                    </code>
                  {:else}
                    <span
                      class="text-sm italic text-slate-500 dark:text-slate-400"
                    >
                      Enter title and artist to see filename preview
                    </span>
                  {/if}
                </div>
                <p class="text-xs text-slate-500 dark:text-slate-400">
                  The file will be renamed using the format: Artist(s) -
                  Title.extension
                </p>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <Dialog.Footer>
      <Button
        variant="outline"
        onclick={() => (open = false)}
        class="cursor-pointer"
      >
        Cancel
      </Button>
      <Button
        onclick={handleSave}
        disabled={!changes.hasSavableChanges}
        class="cursor-pointer"
      >
        Save changes
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
