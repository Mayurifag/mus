<script lang="ts">
  import { trackStore } from "$lib/stores/trackStore";
  import TrackList from "$lib/components/domain/TrackList.svelte";

  $: tracks = $trackStore.tracks;
</script>

<div class="container mx-auto pb-24">
  <TrackList {tracks} />
</div>
