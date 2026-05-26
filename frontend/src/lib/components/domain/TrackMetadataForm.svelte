<script lang="ts">
  import type { ArtworkSearchResult, Track } from "$lib/types";
  import type { AudioMetadata } from "$lib/utils/audioFileAnalyzer";
  import { Input } from "$lib/components/ui/input";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { Button } from "$lib/components/ui/button";
  import { HelpCircle, Plus, X } from "@lucide/svelte";

  import ArtworkPickerButton from "./ArtworkPickerButton.svelte";
  import FilenameDisplay from "./FilenameDisplay.svelte";
  import {
    displayedFilename,
    type ArtistRow,
    type TrackMetadataMode,
  } from "./trackMetadataForm";

  let {
    mode,
    track,
    file,
    metadata,
    title = $bindable(),
    renameFile = $bindable(),
    artists = $bindable(),
    sanitizedTitle,
    sanitizedArtists,
    generatedFilename,
    isFilenameTooLong,
    isFormValid,
    selectedArtwork,
    fallbackArtworkUrl,
    fallbackArtworkAlt,
    onOpenArtwork,
    onAddArtist,
    onRemoveArtist,
  }: {
    mode: TrackMetadataMode;
    track?: Track;
    file?: File;
    metadata?: AudioMetadata;
    title: string;
    renameFile: boolean;
    artists: ArtistRow[];
    sanitizedTitle: string;
    sanitizedArtists: ArtistRow[];
    generatedFilename: string;
    isFilenameTooLong: boolean;
    isFormValid: boolean;
    selectedArtwork: ArtworkSearchResult | null;
    fallbackArtworkUrl?: string | null;
    fallbackArtworkAlt: string;
    onOpenArtwork: () => void;
    onAddArtist: () => void;
    onRemoveArtist: (id: number) => void;
  } = $props();

  const filename = $derived(
    displayedFilename(mode, renameFile, generatedFilename, track),
  );
  const duration = $derived(
    mode === "create"
      ? (metadata?.duration as number | undefined)
      : track?.duration,
  );
</script>

{#snippet filenameHelp()}
  <div class="group relative">
    <HelpCircle
      class="text-muted-foreground hover:text-foreground h-4 w-4 cursor-help transition-colors"
    />
    <div class="absolute top-0 left-6 z-50 hidden group-hover:block">
      <div
        class="bg-popover text-popover-foreground w-64 rounded-md border p-2 text-xs shadow-md"
      >
        Invalid characters for filenames (&lt;, &gt;, :, ", /, \, |, ?, *) are
        automatically removed.
      </div>
    </div>
  </div>
{/snippet}

<div class="grid gap-6 py-6">
  <div class="grid grid-cols-1 gap-6 md:grid-cols-[200px_1fr]">
    <div class="space-y-3">
      <ArtworkPickerButton
        {selectedArtwork}
        fallbackUrl={fallbackArtworkUrl}
        fallbackAlt={fallbackArtworkAlt}
        onOpen={onOpenArtwork}
      />
    </div>

    <div class="space-y-6">
      <div class="space-y-2">
        <div class="flex items-center gap-2">
          <label for="title" class="text-sm font-medium">Title</label>
          {@render filenameHelp()}
        </div>
        <Input
          id="title"
          bind:value={title}
          placeholder="Enter track title"
          class={!isFormValid && sanitizedTitle.trim() === ""
            ? "border-red-500"
            : ""}
        />
      </div>

      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium">Artists</span>
            {@render filenameHelp()}
          </div>
          <Button
            variant="outline"
            size="sm"
            onclick={onAddArtist}
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
                class={!isFormValid &&
                index === 0 &&
                sanitizedArtists[index]?.value.trim() === ""
                  ? "border-red-500"
                  : ""}
              />
              {#if artists.length > 1}
                <Button
                  variant="outline"
                  size="sm"
                  onclick={() => onRemoveArtist(artist.id)}
                  class="h-10 w-10 flex-shrink-0 p-0"
                >
                  <X class="h-4 w-4" />
                </Button>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <FilenameDisplay
        {filename}
        fileSize={mode === "create" ? file?.size : undefined}
        {duration}
      />

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
        <div class="flex items-center space-x-2">
          <Checkbox id="rename-file" bind:checked={renameFile} />
          <label for="rename-file" class="text-sm">
            Rename file to match title
          </label>
        </div>
      {/if}
    </div>
  </div>
</div>
