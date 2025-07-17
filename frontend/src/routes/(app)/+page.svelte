<script lang="ts">
  import TrackList from "$lib/components/domain/TrackList.svelte";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";
</script>

<div
  class="container mx-auto"
  style="padding-bottom: calc(var(--footer-height-mobile) + var(--safe-area-inset-bottom) + 1rem);"
>
  <TrackList audioService={$audioServiceStore} />
</div>

<style>
  @media (min-width: 700px) {
    div {
      padding-bottom: calc(
        var(--footer-height-desktop) + var(--safe-area-inset-bottom) + 1rem
      ) !important;
    }
  }
</style>
