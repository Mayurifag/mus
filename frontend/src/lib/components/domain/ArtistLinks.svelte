<script lang="ts">
  import { trackStore } from "$lib/stores/trackStore";
  import { parseArtists } from "$lib/utils/formatters";

  let {
    artist,
    class: className = "",
  }: {
    artist: string;
    class?: string;
  } = $props();

  const artists = $derived(parseArtists(artist));

  function selectArtist(event: MouseEvent, artistName: string) {
    event.stopPropagation();
    trackStore.setArtistFilter(artistName);
  }
</script>

{#if artists.length > 0}
  {#each artists as artistName, index (artistName)}
    <button
      type="button"
      class="hover:text-accent focus-visible:ring-accent rounded-sm underline-offset-2 transition-colors hover:underline focus-visible:ring-1 focus-visible:outline-none {className}"
      title="Show {artistName} songs"
      aria-label="Show {artistName} songs"
      onclick={(event) => selectArtist(event, artistName)}
      onmousedown={(event) => event.stopPropagation()}
      onmouseup={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      {artistName}
    </button>{#if index < artists.length - 1},
    {/if}
  {/each}
{:else}
  <span class={className}>Unknown Artist</span>
{/if}
