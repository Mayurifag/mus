<script lang="ts">
  import type { Track } from "$lib/types";
  import { onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { Button } from "$lib/components/ui/button";
  import { updateTrack, uploadTrack } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import { Plus, X, Clock, HelpCircle } from "@lucide/svelte";

  import TrackChangesPanel from "./TrackChangesPanel.svelte";
  import FilenameDisplay from "./FilenameDisplay.svelte";

  import type { AudioMetadata } from "$lib/utils/frontendCoverExtractor";

  let {
    open = $bindable(),
    mode,
    track,
    file,
    suggestedTitle,
    suggestedArtist,
    coverDataUrl,
    metadata,
    allTags,
  }: {
    open: boolean;
    mode: "edit" | "create";
    track?: Track;
    file?: File;
    suggestedTitle?: string;
    suggestedArtist?: string;
    coverDataUrl?: string | null;
    metadata?: AudioMetadata;
    allTags?: Record<string, unknown>;
  } = $props();

  let hasTrackChanges = $state(false);
  let trackChangesCount = $state(0);
  let saveOnlyEssentials = $state(true);
  let editableAllTags = $state("");

  function buildDisplayTags(): Record<string, string[]> {
    const baseTags: Record<string, string[]> = { ...(allTags?.v2 || {}) };

    // Add title if available
    if (formState.title.trim()) {
      baseTags.TIT2 = [formState.title.trim()];
    }

    // Add artists if available
    const currentArtistString = artists
      .map((a) => a.value.trim())
      .filter((v) => v)
      .join(", ");
    if (currentArtistString) {
      baseTags.TPE1 = [currentArtistString];
    }

    return baseTags;
  }

  function buildEditableTags(): Record<string, string[]> {
    const baseTags: Record<string, string[]> = { ...(allTags?.v2 || {}) };

    // Remove fields that should only be managed by the system or form fields
    delete baseTags.TIT2; // Title - managed by form
    delete baseTags.TPE1; // Artists - managed by form
    delete baseTags.TLEN; // Length/Duration - managed by system

    return baseTags;
  }
  let isUploading = $state(false);

  let formState = $state({
    title: "",
    renameFile: true,
  });

  let artistIdCounter = 0;
  let artists = $state<{ id: number; value: string }[]>([]);

  // Generate filename based on artists and title
  const generatedFilename = $derived(() => {
    const artistNames = artists
      .filter((a) => a.value.trim())
      .map((a) => a.value.trim())
      .join(", ");
    const title = formState.title.trim();

    if (!artistNames && !title) return file?.name || "untitled.mp3";
    if (!artistNames) return `${title}.mp3`;
    if (!title) return `${artistNames}.mp3`;

    return `${artistNames} - ${title}.mp3`;
  });

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

    // Update editable tags when allTags changes (excluding TIT2/TPE1)
    editableAllTags = JSON.stringify(buildEditableTags(), null, 2);
  }

  onMount(() => {
    resetState();
  });

  // Update editable tags when form values change
  $effect(() => {
    // Only update if user hasn't manually edited the JSON
    const expectedEditableTags = JSON.stringify(buildEditableTags(), null, 2);
    if (!editableAllTags || editableAllTags === expectedEditableTags) {
      editableAllTags = expectedEditableTags;
    }
  });

  function handleOpenChange(newOpen: boolean) {
    if (!newOpen) {
      open = false;
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

  function updateArtist(id: number, value: string) {
    const artist = artists.find((a) => a.id === id);
    if (artist) {
      artist.value = value;
    }
  }

  const currentArtistString = $derived(
    artists
      .map((a) => a.value.trim())
      .filter(Boolean)
      .join("; "),
  );

  const changes = $derived.by(() => {
    if (mode === "create") {
      const hasTitle = formState.title.trim().length > 0;
      const hasPrimaryArtist =
        artists.length > 0 && artists[0].value.trim().length > 0;
      return {
        isFormValid: hasTitle && hasPrimaryArtist,
        hasSavableChanges: hasTitle && hasPrimaryArtist,
      };
    }

    if (!track) return { isFormValid: false, hasSavableChanges: false };

    const titleChanged = formState.title !== track.title;
    const artistChanged = currentArtistString !== track.artist;
    const hasTitle = formState.title.trim().length > 0;
    const hasPrimaryArtist =
      artists.length > 0 && artists[0].value.trim().length > 0;

    return {
      isFormValid: hasTitle && hasPrimaryArtist,
      hasSavableChanges:
        (titleChanged || artistChanged || formState.renameFile) &&
        hasTitle &&
        hasPrimaryArtist,
    };
  });

  $effect(() => {
    if (mode === "edit" && track) {
      hasTrackChanges = changes.hasSavableChanges;
    }
  });

  async function handleSave() {
    if (mode === "create" && file) {
      await handleUpload();
    } else if (mode === "edit" && track) {
      await handleUpdate();
    }
  }

  async function handleUpload() {
    if (!file) return;

    isUploading = true;
    try {
      // Parse edited raw tags if provided
      let parsedRawTags = undefined;
      if (!saveOnlyEssentials) {
        try {
          // Start with form-derived tags (including TIT2/TPE1)
          const finalTags = buildDisplayTags();

          // Merge user's editable tags if provided
          if (editableAllTags.trim()) {
            const userTags = JSON.parse(editableAllTags);
            Object.assign(finalTags, userTags);
          }

          // Reconstruct the full structure with v2 container
          const fullStructure = {
            v2: finalTags,
            raw: { v2: finalTags },
          };
          parsedRawTags = JSON.stringify(fullStructure);
        } catch {
          toast.error("Invalid JSON in raw tags");
          isUploading = false;
          return;
        }
      }

      await uploadTrack(file, formState.title, currentArtistString, {
        saveOnlyEssentials,
        rawTags: parsedRawTags,
      });
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
      open = false;
    } catch (error) {
      console.error("Error updating track:", error);
      toast.error("Failed to update track");
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
            <label for="title" class="text-sm font-medium">Title</label>
            <Input
              id="title"
              bind:value={formState.title}
              placeholder="Enter track title"
              class={!changes.isFormValid && formState.title.trim() === ""
                ? "border-red-500"
                : ""}
            />
          </div>

          <!-- Artists section -->
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-sm font-medium">Artists</span>
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
                    onchange={(e) =>
                      updateArtist(
                        artist.id,
                        (e.target as HTMLInputElement).value,
                      )}
                    placeholder={index === 0
                      ? "Primary artist (required)"
                      : "Additional artist"}
                    class={!changes.isFormValid &&
                    index === 0 &&
                    artist.value.trim() === ""
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
              ? generatedFilename()
              : track?.file_path.split("/").pop() || ""}
            fileSize={mode === "create" ? file?.size : undefined}
            duration={mode === "create" ? metadata?.duration : track?.duration}
          />

          {#if mode === "create"}
            <!-- Save only essentials checkbox -->
            <div class="flex items-center space-x-2">
              <Checkbox
                id="save-essentials"
                bind:checked={saveOnlyEssentials}
              />
              <label for="save-essentials" class="text-sm">
                Save only essential metadata (artists, title, cover)
              </label>
            </div>

            <!-- All tags -->
            {#if !saveOnlyEssentials}
              <div class="bg-muted rounded-lg p-4">
                <div class="mb-2 flex items-center gap-2">
                  <h3 class="font-medium">All tags</h3>
                  <div class="group relative">
                    <button
                      type="button"
                      class="text-muted-foreground hover:text-foreground transition-colors"
                    >
                      <HelpCircle class="h-4 w-4" />
                    </button>
                    <div
                      class="absolute top-0 left-6 z-50 hidden group-hover:block"
                    >
                      <div
                        class="bg-popover text-popover-foreground w-80 rounded-md border p-3 text-sm shadow-md"
                      >
                        <div class="space-y-2">
                          <div class="font-medium">Raw metadata tags</div>
                          <div class="text-xs">
                            ID3v2 metadata tags in JSON format. Edit directly to
                            modify any field not available in the form above.
                            TIT2 (title), TPE1 (artists), and TLEN (duration)
                            are managed automatically.
                          </div>
                          <div class="text-xs">
                            <div class="mb-1 font-medium">Examples:</div>
                            <div class="space-y-1 font-mono text-[10px]">
                              <div>"TALB": ["My Album"]</div>
                              <div>"TYER": ["2024"]</div>
                              <div>"TCOM": ["John Doe"]</div>
                              <div>"TBPM": ["120"]</div>
                              <div>"COMM": ["Live recording"]</div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
                <textarea
                  bind:value={editableAllTags}
                  class="bg-background h-40 w-full resize-y rounded border p-2 font-mono text-xs"
                  placeholder="Raw metadata JSON..."
                ></textarea>
              </div>
            {/if}
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

      {#if mode === "edit" && track && hasTrackChanges}
        <!-- Track changes panel (only in edit mode) -->
        <TrackChangesPanel
          trackId={track.id}
          bind:hasChanges={hasTrackChanges}
          bind:changesCount={trackChangesCount}
        />
      {/if}
    </div>

    <Dialog.Footer class="!justify-between">
      {#if mode === "edit" && !hasTrackChanges}
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
              : mode === "create"
                ? "Upload and save file"
                : "Save changes"}
        >
          {#if isUploading}
            Uploading...
          {:else if mode === "create"}
            Upload and Save
          {:else}
            Save changes
          {/if}
        </Button>
      </div>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
