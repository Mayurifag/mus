<script lang="ts">
  import type { ArtworkSearchResult, AudioCut, Track } from "$lib/types";
  import { onDestroy, onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import {
    createTrackWithUrls,
    updateTrack,
    uploadTrack,
    deleteTrack,
    preloadArtworkSearch,
    trackUpdatesConnectionStatus,
  } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import { permissionsStore } from "$lib/stores/permissionsStore";
  import { artistCountsStore, trackStore } from "$lib/stores/trackStore";
  import { parseArtists } from "$lib/utils/formatters";
  import { categoryLabels, categoryNames } from "$lib/utils/categoryMarkers";
  import { Checkbox } from "$lib/components/ui/checkbox";

  import ArtworkSearchModal from "./ArtworkSearchModal.svelte";
  import TrackDeleteDialog from "./TrackDeleteDialog.svelte";
  import TrackMetadataFooter from "./TrackMetadataFooter.svelte";
  import TrackMetadataForm from "./TrackMetadataForm.svelte";
  import TrackCutEditor from "./TrackCutEditor.svelte";
  import {
    buildTrackUpdatePayload,
    existingTitleSuggestions,
    formatArtistString,
    isTrackMetadataFormValid,
    previewFilename,
    sanitizeArtistRows,
    sanitizeMetadataInput,
    type ArtistRow,
  } from "./trackMetadataForm";

  import type { AudioMetadata } from "$lib/utils/audioFileAnalyzer";

  let {
    open = $bindable(),
    mode,
    track,
    file,
    suggestedTitle,
    suggestedArtist,
    suggestedTags = [],
    sourceDuration,
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
    suggestedTags?: string[];
    sourceDuration?: number | null;
    coverDataUrl?: string | null;
    metadata?: AudioMetadata;
    isDownload?: boolean;
    onDownloadConfirm?: (
      title: string,
      artist: string,
      tags: string[],
      artworkUrl?: string,
    ) => Promise<void>;
    onClose?: () => void;
  } = $props();

  let confirmDeleteOpen = $state(false);
  let isUploading = $state(false);
  let artworkSearchOpen = $state(false);
  let selectedArtwork = $state<ArtworkSearchResult | null>(null);
  let cancelArtworkPreload: (() => void) | null = null;
  let title = $state("");
  let renameFile = $state(true);
  let artistIdCounter = 0;
  let artists = $state<ArtistRow[]>([]);
  let selectedTags = $state<string[]>([]);
  let cut = $state<AudioCut | null>(null);
  let cutValid = $state(true);

  const generatedFilename = $derived(
    previewFilename(
      sanitizeArtistRows(artists),
      sanitizeMetadataInput(title),
      file?.name,
    ),
  );
  const isFilenameTooLong = $derived(generatedFilename.length > 255);
  const hasSourceDuration = $derived(
    typeof sourceDuration === "number" &&
      Number.isFinite(sourceDuration) &&
      sourceDuration > 0,
  );
  const currentArtistString = $derived(formatArtistString(artists));
  const originalTags = $derived(
    (track?.tags ?? [])
      .map((tag) => tag.name)
      .filter((tag) => categoryNames.includes(tag)),
  );
  const tagsChanged = $derived(
    selectedTags.length !== originalTags.length ||
      selectedTags.some((tag) => !originalTags.includes(tag)),
  );
  const updatePayload = $derived({
    ...buildTrackUpdatePayload({
      track,
      title,
      artist: currentArtistString,
      renameFile,
      filename: generatedFilename,
      artworkUrl: selectedArtwork?.image_url,
    }),
    ...(mode === "edit" && tagsChanged ? { tags: selectedTags } : {}),
    ...(cut ? { remove_section: cut } : {}),
  });
  const isFormValid = $derived(
    isTrackMetadataFormValid({
      mode,
      title,
      primaryArtist: artists[0]?.value,
      renameFile,
      filenameTooLong: isFilenameTooLong,
    }),
  );
  const hasSavableChanges = $derived(
    isFormValid &&
      cutValid &&
      (mode === "create" || Object.keys(updatePayload).length > 0),
  );
  const downloadUnavailable = $derived(
    isDownload && $trackUpdatesConnectionStatus !== "connected",
  );

  const fallbackArtworkUrl = $derived.by(() => {
    if (mode === "edit" && track?.has_cover) {
      return track.cover_original_url;
    }
    if (mode === "create") {
      return coverDataUrl ?? null;
    }
    return null;
  });

  const fallbackArtworkAlt = $derived.by(() => {
    if (mode === "edit") {
      return track?.has_cover ? "Track cover" : "No cover";
    }
    if (coverDataUrl) {
      return isDownload ? "Track thumbnail" : "Extracted cover art";
    }
    return "No cover";
  });

  function artistRows(value: string | undefined) {
    const artistList = parseArtists(value ?? "");
    return (artistList.length ? artistList : [""]).map((value) => ({
      id: artistIdCounter++,
      value,
    }));
  }

  function resetState() {
    selectedArtwork = null;
    renameFile = mode === "edit";
    if (mode === "edit" && track) {
      title = track.title;
      artists = artistRows(track.artist);
      selectedTags = (track.tags ?? [])
        .map((tag) => tag.name)
        .filter((tag) => categoryNames.includes(tag));
    } else if (mode === "create" && isDownload) {
      title = suggestedTitle ?? "";
      artists = artistRows(suggestedArtist);
      selectedTags = suggestedTags.filter((tag) => categoryNames.includes(tag));
    } else if (mode === "create" && file) {
      title =
        suggestedTitle || metadata?.title || file.name.replace(/\.[^/.]+$/, "");
      artists = artistRows(
        suggestedArtist?.trim() ? suggestedArtist : metadata?.artist,
      );
    }
  }

  onMount(() => {
    resetState();
    if (mode === "edit" && track) {
      cancelArtworkPreload = preloadArtworkSearch({
        title: track.title,
        artist: track.artist,
      });
    }
  });

  onDestroy(() => {
    cancelArtworkPreload?.();
    artworkSearchOpen = false;
  });

  function handleOpenChange(newOpen: boolean) {
    if (!newOpen) closeModal();
  }

  function hideModal() {
    artworkSearchOpen = false;
    open = false;
  }

  function closeModal() {
    cancelArtworkPreload?.();
    hideModal();
    if (onClose) {
      onClose();
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

  async function handleSave() {
    if (!hasSavableChanges || isUploading || downloadUnavailable) return;
    if (mode === "edit") {
      await handleUpdate();
    } else if (isDownload) {
      await handleDownloadConfirm();
    } else {
      await handleUpload();
    }
  }

  async function handleDownloadConfirm() {
    if (!onDownloadConfirm) return;

    isUploading = true;
    try {
      await onDownloadConfirm(
        title,
        currentArtistString,
        selectedTags,
        selectedArtwork?.image_url,
      );
      hideModal();
    } catch (error) {
      console.error("Error confirming download:", error);
      toast.error("Failed to start download");
    } finally {
      isUploading = false;
    }
  }

  async function handleUpload() {
    if (!file) return;

    isUploading = true;
    try {
      await uploadTrack(
        file,
        title,
        currentArtistString,
        selectedArtwork?.image_url,
      );
      toast.success("File uploaded successfully");
      hideModal();
    } catch (error) {
      console.error("Error uploading file:", error);
      toast.error("Failed to upload file");
    } finally {
      isUploading = false;
    }
  }

  async function handleUpdate() {
    if (!track) return;

    isUploading = true;
    try {
      const result = await updateTrack(track.id, updatePayload);
      if (result.track) {
        trackStore.updateTrack(createTrackWithUrls(result.track));
      }
      if (result.status === "queued") {
        toast.info("Updating track...");
      }
      closeModal();
    } catch (error) {
      console.error("Error updating track:", error);
      toast.error("Failed to update track");
    } finally {
      isUploading = false;
    }
  }

  async function handleDelete() {
    if (!track) return;

    try {
      await deleteTrack(track.id);
      confirmDeleteOpen = false;
      hideModal();
    } catch (error) {
      console.error("Error deleting track:", error);
      toast.error("Failed to delete track");
    }
  }

  function handleArtworkSelect(result: ArtworkSearchResult) {
    selectedArtwork = result;
  }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Content
    class="max-h-[95vh] overflow-y-auto sm:max-w-[700px] lg:max-w-[1000px]"
  >
    <TrackMetadataForm
      {mode}
      {track}
      {file}
      {metadata}
      {sourceDuration}
      artistCounts={isDownload ? $artistCountsStore : {}}
      titleSuggestions={isDownload
        ? existingTitleSuggestions(
            title,
            artists,
            sourceDuration,
            $trackStore.tracks,
          )
        : []}
      bind:title
      bind:renameFile
      bind:artists
      {generatedFilename}
      {isFilenameTooLong}
      {isFormValid}
      {selectedArtwork}
      {fallbackArtworkUrl}
      {fallbackArtworkAlt}
      onOpenArtwork={() => (artworkSearchOpen = true)}
      onAddArtist={addArtist}
      onRemoveArtist={removeArtist}
    />

    {#if isDownload}
      <div class="text-muted-foreground space-y-1 text-sm" role="note">
        <p>
          Review artist credits and recording markers such as Acoustic or Live.
        </p>
        {#if artists.filter((artist) => artist.value.trim()).length > 1}
          <p>
            Multiple credits may include other contributors; check each one.
          </p>
        {/if}
        {#if hasSourceDuration}
          <p>Similar durations do not prove recordings are the same.</p>
        {:else}
          <p>
            Source duration is unavailable; check the recording version
            manually.
          </p>
        {/if}
      </div>
      {#if downloadUnavailable}
        <p class="text-muted-foreground text-sm" role="status">
          Waiting for live updates before starting the download.
        </p>
      {/if}
    {/if}

    {#if isDownload || mode === "edit"}
      <fieldset class="space-y-2">
        <legend class="text-sm font-medium">Categories</legend>
        <div class="flex flex-wrap gap-4">
          {#each categoryNames as tag (tag)}
            <div class="flex items-center gap-2">
              <Checkbox
                id="{isDownload ? 'download' : 'track'}-category-{tag}"
                checked={selectedTags.includes(tag)}
                onCheckedChange={(checked) => {
                  selectedTags = checked
                    ? [...selectedTags, tag]
                    : selectedTags.filter((value) => value !== tag);
                }}
              />
              <label
                for="{isDownload ? 'download' : 'track'}-category-{tag}"
                class="text-sm">{categoryLabels[tag]}</label
              >
            </div>
          {/each}
        </div>
      </fieldset>
    {/if}

    {#if mode === "edit" && track && $permissionsStore.can_write_music_files}
      <TrackCutEditor
        {track}
        onChange={(nextCut, valid) => {
          cut = nextCut;
          cutValid = valid;
        }}
        disabled={isUploading}
      />
    {/if}

    <TrackMetadataFooter
      {mode}
      canDelete={$permissionsStore.can_write_music_files}
      {isUploading}
      {isFormValid}
      {hasSavableChanges}
      saveDisabled={downloadUnavailable}
      hasCut={cut !== null}
      {cutValid}
      onCancel={closeModal}
      onSave={handleSave}
      onDeleteClick={() => (confirmDeleteOpen = true)}
    />
  </Dialog.Content>
</Dialog.Root>

<ArtworkSearchModal
  bind:open={artworkSearchOpen}
  {title}
  artist={currentArtistString}
  selectedUrl={selectedArtwork?.image_url ?? null}
  onSelect={handleArtworkSelect}
/>

<TrackDeleteDialog bind:open={confirmDeleteOpen} onDelete={handleDelete} />
