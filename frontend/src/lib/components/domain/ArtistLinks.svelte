<script lang="ts">
  import { artistCountsStore, trackStore } from "$lib/stores/trackStore";
  import { selectArtistFilter } from "$lib/utils/artistFilterNavigation";
  import { parseArtists } from "$lib/utils/formatters";

  let {
    artist,
    class: className = "",
  }: {
    artist: string;
    class?: string;
  } = $props();

  const artists = $derived([...new Set(parseArtists(artist))]);
  const selectedArtist = $derived($trackStore.selectedArtist);
  const artistCounts = $derived($artistCountsStore);

  function canSelectArtist(artistName: string): boolean {
    return selectedArtist !== artistName && (artistCounts[artistName] ?? 0) > 1;
  }

  function selectArtist(event: MouseEvent, artistName: string) {
    event.stopPropagation();
    selectArtistFilter(artistName);
  }
</script>

{#if artists.length > 0}
  {#each artists as artistName, index (artistName)}
    {#if canSelectArtist(artistName)}
      <button
        type="button"
        class="hover:text-accent focus-visible:ring-accent cursor-pointer rounded-sm underline-offset-2 transition-colors hover:underline focus-visible:ring-1 focus-visible:outline-none {className}"
        title="Show {artistName} songs"
        aria-label="Show {artistName} songs"
        onclick={(event) => selectArtist(event, artistName)}
        onmousedown={(event) => event.stopPropagation()}
        onmouseup={(event) => event.stopPropagation()}
        onkeydown={(event) => event.stopPropagation()}
      >
        {artistName}
      </button>
    {:else}
      <span class={className}>{artistName}</span>
    {/if}{#if index < artists.length - 1},&nbsp;
    {/if}
  {/each}
{:else}
  <span class={className}>Unknown Artist</span>
{/if}
