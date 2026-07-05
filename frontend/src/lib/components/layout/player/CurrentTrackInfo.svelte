<script lang="ts">
  import ArtistLinks from "$lib/components/domain/ArtistLinks.svelte";
  import type { Track } from "$lib/types";
  import { displayTrackTitle } from "$lib/utils/categoryMarkers";

  let {
    track,
    mobile = false,
  }: {
    track: Track | null;
    mobile?: boolean;
  } = $props();

  const trackTitle = $derived(track ? displayTrackTitle(track) : "");
</script>

{#if mobile}
  <div class="-mt-1 flex w-full items-center justify-center px-4 text-center">
    {#if track}
      <span class="text-muted-foreground truncate text-sm font-medium">
        <ArtistLinks artist={track.artist} /> - {trackTitle}
      </span>
    {:else}
      <span class="text-muted-foreground text-sm">Not Playing</span>
    {/if}
  </div>
{:else}
  <div class="flex min-w-0 flex-1 items-center">
    {#if track}
      <div
        class="desktop:block desktop:h-24 desktop:w-24 desktop:my-5 desktop:ml-5 my-6 ml-6 hidden h-32 w-32 flex-shrink-0 overflow-hidden rounded-md"
      >
        {#if track.has_cover && track.cover_original_url}
          <img
            src={track.cover_original_url}
            alt="Album Cover"
            class="h-full w-full object-cover"
          />
        {:else}
          <img
            src="/images/no-cover.svg"
            alt="No Album Cover"
            class="h-full w-full object-cover"
          />
        {/if}
      </div>
      <div
        class="sm700:flex ml-4 hidden min-w-0 flex-1 flex-col overflow-hidden"
      >
        <span class="truncate text-base font-medium">{trackTitle}</span>
        <span class="text-muted-foreground truncate text-sm">
          <ArtistLinks artist={track.artist} />
        </span>
      </div>
    {:else}
      <div
        class="desktop:block desktop:h-24 desktop:w-24 desktop:my-5 desktop:ml-5 bg-muted my-6 ml-6 flex hidden h-32 w-32 flex-shrink-0 items-center justify-center rounded-md"
      >
        <span class="text-muted-foreground text-xs">No Track</span>
      </div>
      <div
        class="sm700:flex ml-4 hidden min-w-0 flex-1 flex-col overflow-hidden"
      >
        <span class="text-muted-foreground text-sm">Not Playing</span>
      </div>
    {/if}
  </div>
{/if}
