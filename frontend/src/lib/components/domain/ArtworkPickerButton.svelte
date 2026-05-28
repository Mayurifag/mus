<script lang="ts">
  import type { ArtworkSearchResult } from "$lib/types";

  let {
    selectedArtwork,
    fallbackUrl,
    fallbackAlt,
    onOpen,
  }: {
    selectedArtwork: ArtworkSearchResult | null;
    fallbackUrl?: string | null;
    fallbackAlt: string;
    onOpen: () => void;
  } = $props();

  const imageUrl = $derived(
    selectedArtwork?.thumbnail_url ?? fallbackUrl ?? "/images/no-cover.svg",
  );
  const imageAlt = $derived(selectedArtwork ? "Selected artwork" : fallbackAlt);
</script>

<button
  type="button"
  class="bg-muted group relative aspect-square w-full overflow-hidden rounded-lg text-left"
  onclick={onOpen}
  title="Find artwork"
>
  <img src={imageUrl} alt={imageAlt} class="h-full w-full object-cover" />
  <div
    class="bg-overlay/70 text-overlay-foreground absolute inset-x-0 bottom-0 px-3 py-2 text-center text-xs opacity-0 transition group-hover:opacity-100"
  >
    Find artwork
  </div>
</button>
