<script lang="ts">
  import type { ArtworkSearchResult } from "$lib/types";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { searchArtworkStream } from "$lib/services/apiClient";
  import { toast } from "svelte-sonner";
  import { onDestroy } from "svelte";

  let {
    open = $bindable(),
    title,
    artist,
    selectedUrl,
    onSelect,
  }: {
    open: boolean;
    title: string;
    artist: string;
    selectedUrl?: string | null;
    onSelect: (result: ArtworkSearchResult) => void;
  } = $props();

  let query = $state("");
  let results = $state<ArtworkSearchResult[]>([]);
  let isLoading = $state(false);
  let lastSearch = $state("");
  let wasOpen = $state(false);
  let activeSearch: AbortController | null = null;

  onDestroy(cancelSearch);

  $effect(() => {
    if (open && !wasOpen) {
      startInitialSearch();
    }
    wasOpen = open;
  });

  function handleOpenChange(newOpen: boolean) {
    open = newOpen;
    if (newOpen) {
      startInitialSearch();
    } else {
      cancelSearch();
    }
  }

  function startInitialSearch() {
    query = [title, artist].filter(Boolean).join(" ");
    lastSearch = "";
    void handleSearch(query);
  }

  async function handleSearch(searchQuery = query) {
    const trimmedQuery = searchQuery.trim();
    if (!trimmedQuery || (isLoading && trimmedQuery === lastSearch)) return;

    cancelSearch();
    const controller = new AbortController();
    activeSearch = controller;
    results = [];
    isLoading = true;
    lastSearch = trimmedQuery;
    try {
      await searchArtworkStream({
        title: trimmedQuery,
        artist,
        signal: controller.signal,
        onResults: (nextResults) => {
          if (activeSearch === controller) {
            results = nextResults;
          }
        },
      });
    } catch (error) {
      if (controller.signal.aborted) return;
      console.error("Error searching artwork:", error);
      toast.error("Failed to search artwork");
    } finally {
      if (activeSearch === controller) {
        activeSearch = null;
        isLoading = false;
      }
    }
  }

  function cancelSearch() {
    activeSearch?.abort();
    activeSearch = null;
    isLoading = false;
  }

  function handleSelect(result: ArtworkSearchResult) {
    cancelSearch();
    onSelect(result);
    open = false;
  }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Content class="max-h-[92vh] overflow-y-auto sm:max-w-[860px]">
    <Dialog.Header>
      <Dialog.Title>Find artwork</Dialog.Title>
      <Dialog.Description>
        Pick an image to embed into this track when you save.
      </Dialog.Description>
    </Dialog.Header>

    <div class="space-y-4 py-4">
      <form
        class="flex gap-2"
        onsubmit={(event) => {
          event.preventDefault();
          void handleSearch();
        }}
      >
        <Input bind:value={query} placeholder="Search title and artist" />
        <Button type="submit" disabled={isLoading || !query.trim()}>
          {isLoading ? "Searching..." : "Search"}
        </Button>
      </form>

      {#if results.length > 0}
        <div
          class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5"
        >
          {#each results as result (result.id)}
            <button
              type="button"
              class="group bg-muted hover:ring-primary overflow-hidden rounded-lg border text-left transition hover:ring-2 {selectedUrl ===
              result.image_url
                ? 'ring-primary ring-2'
                : ''}"
              onclick={() => handleSelect(result)}
            >
              <div class="aspect-square overflow-hidden">
                <img
                  src={result.thumbnail_url}
                  alt={result.title || "Artwork result"}
                  class="h-full w-full object-cover transition group-hover:scale-105"
                  loading="lazy"
                  decoding="async"
                />
              </div>
              <div class="space-y-1 p-2">
                <div class="truncate text-xs font-medium">
                  {result.title || "Untitled"}
                </div>
                <div class="text-muted-foreground truncate text-[11px]">
                  {result.artist || result.source}
                </div>
                {#if result.width && result.height}
                  <div class="text-muted-foreground text-[10px]">
                    {result.width}x{result.height}
                  </div>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      {:else if isLoading}
        <div class="text-muted-foreground py-10 text-center text-sm">
          Searching artwork...
        </div>
      {:else if lastSearch}
        <div class="text-muted-foreground py-10 text-center text-sm">
          No artwork found.
        </div>
      {/if}
    </div>
  </Dialog.Content>
</Dialog.Root>
