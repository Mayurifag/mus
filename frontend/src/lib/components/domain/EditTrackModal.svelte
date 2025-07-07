<script lang="ts">
  import type { Track } from "$lib/types";
  import { onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { Button } from "$lib/components/ui/button";
  import { updateTrack } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import { Plus, X, Clock } from "@lucide/svelte";
  import TrackChangesPanel from "./TrackChangesPanel.svelte";

  let {
    open = $bindable(),
    track,
  }: {
    open: boolean;
    track: Track;
  } = $props();

  let hasTrackChanges = $state(false);
  let trackChangesCount = $state(0);

  let formState = $state({
    title: "",
    renameFile: true,
  });

  let artistIdCounter = 0;
  let artists = $state<{ id: number; value: string }[]>([]);

  function resetState() {
    formState.title = track.title;
    formState.renameFile = true;
    const artistList = (track.artist?.split(";") ?? [])
      .map((a) => a.trim())
      .filter(Boolean);

    // Ensure there's always at least one artist field
    if (artistList.length === 0) {
      artistList.push("");
    }

    artists = artistList.map((value) => ({ id: artistIdCounter++, value }));
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
    artists
      .map((a) => a.value.trim())
      .filter((a) => a !== "")
      .join(";"),
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
      .map((a) => a.value.trim())
      .filter((a) => a !== "")
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

    // Validation checks
    const isTitleValid = formState.title.trim() !== "";
    const isPrimaryArtistValid =
      artists.length > 0 && artists[0].value.trim() !== "";
    const isFormValid = isTitleValid && isPrimaryArtistValid;

    const hasSavableChanges =
      isFormValid &&
      (metadataChanged || (formState.renameFile && filenameWouldChange));

    const hasUnsavedChanges = metadataChanged || formState.renameFile !== true;

    return {
      hasSavableChanges,
      hasUnsavedChanges,
      showRenameSection: metadataChanged || filenameWouldChange,
      isFormValid,
      isTitleValid,
      isPrimaryArtistValid,
    };
  });

  function addArtist() {
    artists = [...artists, { id: artistIdCounter++, value: "" }];
  }

  function removeArtist(id: number) {
    // Always keep at least one artist field
    if (artists.length > 1) {
      artists = artists.filter((artist) => artist.id !== id);
    } else {
      // If it's the only artist, just clear its value instead of removing
      artists[0].value = "";
    }
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
  <Dialog.Content
    class="max-h-[95vh] overflow-y-auto sm:max-w-[600px] lg:max-w-[900px]"
  >
    <div class="grid gap-6 py-6">
      <!-- 2-column layout: Cover image + Edit fields -->
      <div class="grid grid-cols-1 gap-6 md:grid-cols-[200px_1fr]">
        <!-- Left column: Cover image with filename tooltip -->
        <div class="space-y-3">
          <div
            class="bg-muted group relative aspect-square w-full overflow-hidden rounded-lg"
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

            <!-- Filename tooltip on hover -->
            <div
              class="absolute bottom-0 left-0 right-0 bg-black/75 p-2 text-xs text-white opacity-0 transition-opacity duration-200 group-hover:opacity-100"
            >
              {originalFilename}
            </div>
          </div>
        </div>

        <!-- Right column: Edit fields -->
        <div class="space-y-4">
          <!-- Title field -->
          <div class="space-y-2">
            <label for="title" class="text-sm font-medium">Title</label>
            <Input
              id="title"
              bind:value={formState.title}
              class={`w-full ${!changes.isTitleValid ? "border-destructive bg-destructive/10" : ""}`}
            />
            {#if !changes.isTitleValid}
              <p class="text-destructive text-sm">Title is required</p>
            {/if}
          </div>

          <!-- Artists fields -->
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
                <div class="space-y-1">
                  <div class="flex items-center gap-2">
                    <Input
                      bind:value={artist.value}
                      placeholder={i === 0
                        ? "Primary artist"
                        : "Additional artist"}
                      class={`flex-1 ${i === 0 && !changes.isPrimaryArtistValid ? "border-destructive bg-destructive/10" : ""}`}
                    />
                    {#if artists.length > 1 || (artists.length === 1 && artist.value.trim() !== "")}
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
                  {#if i === 0 && !changes.isPrimaryArtistValid}
                    <p class="text-destructive text-sm">
                      Primary artist is required
                    </p>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        </div>
      </div>

      {#if changes.showRenameSection && changes.hasSavableChanges}
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
                  {#if newFilenamePreview && changes.isFormValid}
                    <code
                      class="break-all text-sm text-slate-800 dark:text-slate-200"
                    >
                      {newFilenamePreview}
                    </code>
                  {:else if !changes.isFormValid}
                    <span
                      class="text-destructive dark:text-destructive text-sm italic"
                    >
                      Complete required fields (title and primary artist) to see
                      filename preview
                    </span>
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

      <!-- Always render TrackChangesPanel for binding, show conditionally -->
      <div class={hasTrackChanges ? "mt-6 border-t pt-6" : "hidden"}>
        <TrackChangesPanel
          trackId={track.id}
          bind:hasChanges={hasTrackChanges}
          bind:changesCount={trackChangesCount}
        />
      </div>
    </div>

    <Dialog.Footer class="!justify-between">
      {#if !hasTrackChanges}
        <div class="flex items-center gap-2">
          <Clock class="text-muted-foreground h-4 w-4" />
          <span class="text-muted-foreground text-sm"> 0 changes </span>
        </div>
      {:else}
        <div></div>
      {/if}

      <div class="flex gap-2">
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
          title={!changes.isFormValid
            ? "Please fill in required fields (title and primary artist)"
            : !changes.hasSavableChanges
              ? "No changes to save"
              : "Save changes"}
        >
          Save changes
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
