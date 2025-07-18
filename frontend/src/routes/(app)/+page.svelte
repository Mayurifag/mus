<script lang="ts">
  import TrackList from "$lib/components/domain/TrackList.svelte";
  import { audioServiceStore } from "$lib/stores/audioServiceStore";
</script>

<div class="container mx-auto">
  <TrackList audioService={$audioServiceStore} />
</div>
