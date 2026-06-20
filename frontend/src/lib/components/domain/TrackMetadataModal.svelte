<script lang="ts">
  import type { ArtworkSearchResult, Track } from "$lib/types";
  import { onDestroy, onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import {
    createTrackWithUrls,
    updateTrack,
    uploadTrack,
    deleteTrack,
    preloadArtworkSearch,
  } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import { permissionsStore } from "$lib/stores/permissionsStore";
  import { trackStore } from "$lib/stores/trackStore";
  import { parseArtists } from "$lib/utils/formatters";

  import ArtworkSearchModal from "./ArtworkSearchModal.svelte";
  import TrackDeleteDialog from "./TrackDeleteDialog.svelte";
  import TrackMetadataFooter from "./TrackMetadataFooter.svelte";
  import TrackMetadataForm from "./TrackMetadataForm.svelte";
  import {
    buildTrackUpdatePayload,
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
  let selectedTags = $state<string[]>([]);
  let renameFile = $state(true);
  let artistIdCounter = 0;
  let artists = $state<ArtistRow[]>([]);

  const sanitizedTitle = $derived(sanitizeMetadataInput(title));
  const sanitizedArtists = $derived(sanitizeArtistRows(artists));
  const generatedFilename = $derived(
    previewFilename(sanitizedArtists, sanitizedTitle, file?.name),
  );
  const isFilenameTooLong = $derived(generatedFilename.length > 255);
  const currentArtistString = $derived(formatArtistString(sanitizedArtists));
  const updatePayload = $derived(
    buildTrackUpdatePayload({
      track,
      title: sanitizedTitle,
      artist: currentArtistString,
      renameFile,
      filename: generatedFilename,
      artworkUrl: selectedArtwork?.image_url,
      tags: selectedTags,
    }),
  );
  const isFormValid = $derived(
    isTrackMetadataFormValid({
      mode,
      title: sanitizedTitle,
      primaryArtist: sanitizedArtists[0]?.value,
      renameFile,
      filenameTooLong: isFilenameTooLong,
    }),
  );
  const hasSavableChanges = $derived(
    isFormValid && (mode === "create" || Object.keys(updatePayload).length > 0),
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
    renameFile = false;
    if (mode === "edit" && track) {
      title = track.title;
      artists = artistRows(track.artist);
      selectedTags = (track.tags ?? []).map((tag) => tag.name);
    } else if (mode === "create" && isDownload) {
      title = suggestedTitle ?? "";
      artists = artistRows(suggestedArtist);
      selectedTags = suggestedTags;
    } else if (mode === "create" && file) {
      title =
        suggestedTitle || metadata?.title || file.name.replace(/\.[^/.]+$/, "");
      artists = artistRows(
        suggestedArtist?.trim() ? suggestedArtist : metadata?.artist,
      );
      selectedTags = [];
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
        sanitizedTitle,
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
        sanitizedTitle,
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
      bind:title
      bind:selectedTags
      bind:renameFile
      bind:artists
      {sanitizedTitle}
      {sanitizedArtists}
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

    <TrackMetadataFooter
      {mode}
      canDelete={$permissionsStore.can_write_music_files}
      {isUploading}
      {isFormValid}
      {hasSavableChanges}
      onCancel={closeModal}
      onSave={handleSave}
      onDeleteClick={() => (confirmDeleteOpen = true)}
    />
  </Dialog.Content>
</Dialog.Root>

<ArtworkSearchModal
  bind:open={artworkSearchOpen}
  title={sanitizedTitle}
  artist={currentArtistString}
  selectedUrl={selectedArtwork?.image_url ?? null}
  onSelect={handleArtworkSelect}
/>

<TrackDeleteDialog bind:open={confirmDeleteOpen} onDelete={handleDelete} />
