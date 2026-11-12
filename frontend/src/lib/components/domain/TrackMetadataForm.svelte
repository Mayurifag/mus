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
    artistSuggestions,
    type ArtistRow,
    type TrackMetadataMode,
  } from "./trackMetadataForm";

  let {
    mode,
    track,
    file,
    metadata,
    sourceDuration,
    artistCounts = {},
    titleSuggestions = [],
    title = $bindable(),
    renameFile = $bindable(),
    artists = $bindable(),
    generatedFilename,
    filenameBytes,
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
    sourceDuration?: number | null;
    artistCounts?: Record<string, number>;
    titleSuggestions?: string[];
    title: string;
    renameFile: boolean;
    artists: ArtistRow[];
    generatedFilename: string;
    filenameBytes: number;
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
      ? ((metadata?.duration as number | undefined) ??
          (sourceDuration &&
          Number.isFinite(sourceDuration) &&
          sourceDuration > 0
            ? sourceDuration
            : undefined))
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
          class={!isFormValid && title.trim() === ""
            ? "border-destructive"
            : ""}
        />
        {#if titleSuggestions.length > 0}
          <div
            class="text-muted-foreground flex flex-wrap items-center gap-1 text-xs"
            role="group"
            aria-label="Possible existing title spellings"
          >
            <span>Possible existing title (check recording):</span>
            {#each titleSuggestions as suggestion (suggestion)}
              <button
                type="button"
                class="hover:text-foreground rounded border px-2 py-1"
                onclick={() => (title = suggestion)}>{suggestion}</button
              >
            {/each}
          </div>
        {/if}
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
            aria-label="Add artist"
            onclick={onAddArtist}
            class="h-8 w-8 p-0"
          >
            <Plus class="h-4 w-4" />
          </Button>
        </div>

        <div class="space-y-2">
          {#each artists as artist, index (artist.id)}
            {@const suggestions = artistSuggestions(artist.value, artistCounts)}
            <div class="space-y-1">
              <div class="flex gap-2">
                <Input
                  bind:value={artist.value}
                  placeholder={index === 0
                    ? "Primary artist (required)"
                    : "Additional artist"}
                  class={!isFormValid &&
                  index === 0 &&
                  artist.value.trim() === ""
                    ? "border-destructive"
                    : ""}
                />
                {#if artists.length > 1}
                  <Button
                    variant="outline"
                    size="sm"
                    aria-label="Remove artist {index + 1}"
                    onclick={() => onRemoveArtist(artist.id)}
                    class="h-10 w-10 flex-shrink-0 p-0"
                  >
                    <X class="h-4 w-4" />
                  </Button>
                {/if}
              </div>
              {#if suggestions.length > 0}
                <div
                  class="text-muted-foreground flex flex-wrap items-center gap-1 text-xs"
                  role="group"
                  aria-label="Artist suggestions for row {index + 1}"
                >
                  <span>Existing artists:</span>
                  {#each suggestions as suggestion (suggestion)}
                    <button
                      type="button"
                      class="hover:text-foreground rounded border px-2 py-1"
                      onclick={() => (artist.value = suggestion)}
                      >{suggestion} ({artistCounts[suggestion]})</button
                    >
                  {/each}
                </div>
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
        <div
          class="border-destructive/30 bg-destructive/10 rounded-lg border p-3"
        >
          <div class="text-destructive text-sm font-medium">
            Filename is too long (max 255 UTF-8 bytes)
          </div>
          <div class="text-destructive/80 mt-1 text-xs">
            Current length: {filenameBytes} bytes
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
