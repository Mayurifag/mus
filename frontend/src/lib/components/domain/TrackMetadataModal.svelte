<script lang="ts">
  import type { Track } from "$lib/types";
  import { onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { Button } from "$lib/components/ui/button";
  import {
    updateTrack,
    uploadTrack,
    deleteTrack,
  } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import { Plus, X, HelpCircle, Save, Trash2 } from "@lucide/svelte";
  import { permissionsStore } from "$lib/stores/permissionsStore";

  import FilenameDisplay from "./FilenameDisplay.svelte";

  import type { AudioMetadata } from "$lib/utils/audioFileAnalyzer";

  let {
    open = $bindable(),
    mode,
    track,
    file,
    suggestedTitle,
    suggestedArtist,
    coverDataUrl,
    metadata,
    isDownload = false,
    onDownloadConfirm,
    onClose,
  }: {
    open: boolean;
    mode: "edit" | "create";
    track?: Track;
    file?: File;
    suggestedTitle?: string;
    suggestedArtist?: string;
    coverDataUrl?: string | null;
    metadata?: AudioMetadata;
    isDownload?: boolean;
    onDownloadConfirm?: (title: string, artist: string) => Promise<void>;
    onClose?: () => void;
  } = $props();

  let confirmDeleteOpen = $state(false);
  let isUploading = $state(false);

  let formState = $state({
    title: "",
    renameFile: true,
  });

  let artistIdCounter = 0;
  let artists = $state<{ id: number; value: string }[]>([]);

  // Generate filename based on artists and title
  const generatedFilename = $derived.by(() => {
    const artistNames = sanitizedArtists
      .filter((a) => a.value.trim())
      .map((a) => a.value.trim())
      .join(", ");
    const title = sanitizedTitle.trim();

    if (!artistNames && !title) return file?.name || "untitled.mp3";
    if (!artistNames) return `${title}.mp3`;
    if (!title) return `${artistNames}.mp3`;

    return `${artistNames} - ${title}.mp3`;
  });

  // Check if filename is too long
  const isFilenameTooLong = $derived.by(() => {
    return generatedFilename.length > 255;
  });

  // Sanitize input by removing invalid filename characters
  function sanitizeInput(value: string): string {
    return value.replace(/[<>:"/\\|?*]/g, "");
  }

  // Sanitized versions for filename generation
  const sanitizedTitle = $derived(sanitizeInput(formState.title));
  const sanitizedArtists = $derived(
    artists.map((artist) => ({
      ...artist,
      value: sanitizeInput(artist.value),
    })),
  );

  function resetState() {
    if (mode === "edit" && track) {
      formState.title = track.title;
      formState.renameFile = true;
      const artistList = (track.artist?.split(";") ?? [])
        .map((a) => a.trim())
        .filter(Boolean);

      if (artistList.length === 0) {
        artistList.push("");
      }

      artists = artistList.map((value) => ({ id: artistIdCounter++, value }));
    } else if (mode === "create" && file) {
      // Use suggested title and artist from parsed filename
      formState.title =
        suggestedTitle || metadata?.title || file.name.replace(/\.[^/.]+$/, "");
      formState.renameFile = false; // Not applicable for create mode

      // Set up artists with suggested artist or empty field
      if (suggestedArtist && suggestedArtist.trim()) {
        artists = [{ id: artistIdCounter++, value: suggestedArtist.trim() }];
      } else if (metadata?.artist) {
        artists = [{ id: artistIdCounter++, value: metadata.artist }];
      } else {
        artists = [{ id: artistIdCounter++, value: "" }];
      }
    }
  }

  onMount(() => {
    resetState();
  });

  function handleOpenChange(newOpen: boolean) {
    if (!newOpen) {
      open = false;
      if (onClose) {
        onClose();
      }
    }
  }

  function addArtist() {
    artists.push({ id: artistIdCounter++, value: "" });
  }

  function removeArtist(id: number) {
    if (artists.length > 1) {
      artists = artists.filter((artist) => artist.id !== id);
    }
  }

  const currentArtistString = $derived(
    sanitizedArtists
      .map((a) => a.value.trim())
      .filter(Boolean)
      .join("; "),
  );

  const changes = $derived.by(() => {
    if (mode === "create") {
      const hasTitle = sanitizedTitle.trim().length > 0;
      const hasPrimaryArtist =
        sanitizedArtists.length > 0 &&
        sanitizedArtists[0].value.trim().length > 0;
      const filenameValid = !isFilenameTooLong;
      return {
        isFormValid: hasTitle && hasPrimaryArtist && filenameValid,
        hasSavableChanges: hasTitle && hasPrimaryArtist && filenameValid,
      };
    }

    if (!track) return { isFormValid: false, hasSavableChanges: false };

    const titleChanged = sanitizedTitle !== track.title;
    const artistChanged = currentArtistString !== track.artist;
    const hasTitle = sanitizedTitle.trim().length > 0;
    const hasPrimaryArtist =
      sanitizedArtists.length > 0 &&
      sanitizedArtists[0].value.trim().length > 0;
    const filenameValid = !formState.renameFile || !isFilenameTooLong;

    return {
      isFormValid: hasTitle && hasPrimaryArtist && filenameValid,
      hasSavableChanges:
        (titleChanged || artistChanged) &&
        hasTitle &&
        hasPrimaryArtist &&
        filenameValid,
    };
  });

  async function handleSave() {
    if (mode === "create" && file) {
      await handleUpload();
    } else if (mode === "create" && isDownload && onDownloadConfirm) {
      await handleDownloadConfirm();
    } else if (mode === "edit" && track) {
      await handleUpdate();
    }
  }

  async function handleDownloadConfirm() {
    if (!onDownloadConfirm) return;

    isUploading = true;
    try {
      await onDownloadConfirm(sanitizedTitle, currentArtistString);
      toast.success("Track added successfully");
      open = false;
    } catch (error) {
      console.error("Error confirming download:", error);
      toast.error("Failed to add track");
    } finally {
      isUploading = false;
    }
  }

  async function handleUpload() {
    if (!file) return;

    isUploading = true;
    try {
      await uploadTrack(file, sanitizedTitle, currentArtistString);
      toast.success("File uploaded successfully");
      open = false;
    } catch (error) {
      console.error("Error uploading file:", error);
      toast.error("Failed to upload file");
    } finally {
      isUploading = false;
    }
  }

  async function handleUpdate() {
    if (!track) return;

    const payload: { title?: string; artist?: string; rename_file?: boolean } =
      {};

    if (sanitizedTitle !== track.title) {
      payload.title = sanitizedTitle;
    }
    if (currentArtistString !== track.artist) {
      payload.artist = currentArtistString;
    }
    if (formState.renameFile) {
      payload.rename_file = true;
    }

    try {
      await updateTrack(track.id, payload);
      open = false;
    } catch (error) {
      console.error("Error updating track:", error);
      toast.error("Failed to update track");
    }
  }

  async function handleDelete() {
    if (!track) return;

    try {
      await deleteTrack(track.id);
      confirmDeleteOpen = false;
      open = false;
    } catch (error) {
      console.error("Error deleting track:", error);
      toast.error("Failed to delete track");
    }
  }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Content
    class="max-h-[95vh] overflow-y-auto sm:max-w-[700px] lg:max-w-[1000px]"
  >
    <div class="grid gap-6 py-6">
      <!-- 2-column layout: Cover image + Edit fields -->
      <div class="grid grid-cols-1 gap-6 md:grid-cols-[200px_1fr]">
        <!-- Left column: Cover image or file info -->
        <div class="space-y-3">
          {#if mode === "edit" && track}
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
            </div>
          {:else if mode === "create" && file}
            <!-- Cover image if available -->
            {#if coverDataUrl}
              <div
                class="bg-muted group relative aspect-square w-full overflow-hidden rounded-lg"
              >
                <img
                  src={coverDataUrl}
                  alt="Extracted cover art"
                  class="h-full w-full object-cover"
                />
              </div>
            {/if}
          {/if}
        </div>

        <!-- Right column: Form fields -->
        <div class="space-y-6">
          <!-- Title field -->
          <div class="space-y-2">
            <div class="flex items-center gap-2">
              <label for="title" class="text-sm font-medium">Title</label>
              <div class="group relative">
                <HelpCircle
                  class="text-muted-foreground hover:text-foreground h-4 w-4 cursor-help transition-colors"
                />
                <div
                  class="absolute top-0 left-6 z-50 hidden group-hover:block"
                >
                  <div
                    class="bg-popover text-popover-foreground w-64 rounded-md border p-2 text-xs shadow-md"
                  >
                    Invalid characters for filenames (&lt;, &gt;, :, ", /, \, |,
                    ?, *) are automatically removed.
                  </div>
                </div>
              </div>
            </div>
            <Input
              id="title"
              bind:value={formState.title}
              placeholder="Enter track title"
              class={!changes.isFormValid && sanitizedTitle.trim() === ""
                ? "border-red-500"
                : ""}
            />
          </div>

          <!-- Artists section -->
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium">Artists</span>
                <div class="group relative">
                  <HelpCircle
                    class="text-muted-foreground hover:text-foreground h-4 w-4 cursor-help transition-colors"
                  />
                  <div
                    class="absolute top-0 left-6 z-50 hidden group-hover:block"
                  >
                    <div
                      class="bg-popover text-popover-foreground w-64 rounded-md border p-2 text-xs shadow-md"
                    >
                      Invalid characters for filenames (&lt;, &gt;, :, ", /, \,
                      |, ?, *) are automatically removed.
                    </div>
                  </div>
                </div>
              </div>
              <Button
                variant="outline"
                size="sm"
                onclick={addArtist}
                class="h-8 w-8 p-0"
              >
                <Plus class="h-4 w-4" />
              </Button>
            </div>

            <div class="space-y-2">
              {#each artists as artist, index (artist.id)}
                <div class="flex gap-2">
                  <Input
                    bind:value={artist.value}
                    placeholder={index === 0
                      ? "Primary artist (required)"
                      : "Additional artist"}
                    class={!changes.isFormValid &&
                    index === 0 &&
                    sanitizedArtists[index]?.value.trim() === ""
                      ? "border-red-500"
                      : ""}
                  />
                  {#if artists.length > 1}
                    <Button
                      variant="outline"
                      size="sm"
                      onclick={() => removeArtist(artist.id)}
                      class="h-10 w-10 flex-shrink-0 p-0"
                    >
                      <X class="h-4 w-4" />
                    </Button>
                  {/if}
                </div>
              {/each}
            </div>
          </div>

          <!-- Filename display -->
          <FilenameDisplay
            filename={mode === "create"
              ? generatedFilename
              : formState.renameFile
                ? generatedFilename
                : track?.file_path.split("/").pop() || ""}
            fileSize={mode === "create" ? file?.size : undefined}
            duration={mode === "create"
              ? (metadata?.duration as number | undefined)
              : track?.duration}
          />

          <!-- Filename length warning -->
          {#if isFilenameTooLong}
            <div class="rounded-lg border border-red-200 bg-red-50 p-3">
              <div class="text-sm font-medium text-red-800">
                Filename is too long (max 255 characters)
              </div>
              <div class="mt-1 text-xs text-red-600">
                Current length: {generatedFilename.length} characters
              </div>
            </div>
          {/if}

          {#if mode === "edit"}
            <!-- Rename file checkbox (only in edit mode) -->
            <div class="flex items-center space-x-2">
              <Checkbox id="rename-file" bind:checked={formState.renameFile} />
              <label for="rename-file" class="text-sm">
                Rename file to match title
              </label>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <Dialog.Footer class="!justify-between">
      <div class="mr-auto">
        {#if mode === "edit" && $permissionsStore.can_write_music_files}
          <Button
            variant="outline"
            size="sm"
            onclick={() => (confirmDeleteOpen = true)}
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
          onclick={() => (open = false)}
          class="cursor-pointer"
          disabled={isUploading}
        >
          Cancel
        </Button>
        <Button
          onclick={handleSave}
          disabled={!changes.hasSavableChanges || isUploading}
          class="cursor-pointer"
          title={!changes.isFormValid
            ? "Please fill in required fields (title and primary artist)"
            : !changes.hasSavableChanges
              ? "No changes to save"
              : "Save"}
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
  </Dialog.Content>
</Dialog.Root>

<!-- Delete confirmation dialog -->
<Dialog.Root bind:open={confirmDeleteOpen}>
  <Dialog.Content class="sm:max-w-[425px]">
    <Dialog.Header>
      <Dialog.Title>Are you sure?</Dialog.Title>
      <Dialog.Description>
        This action cannot be undone. This will permanently delete the track
        from your library and remove the associated files from your system.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (confirmDeleteOpen = false)}>
        Cancel
      </Button>
      <Button variant="destructive" onclick={handleDelete}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
