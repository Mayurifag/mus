<script lang="ts">
  import { Upload } from "@lucide/svelte";

  let {
    visible = false,
  }: {
    visible: boolean;
  } = $props();
</script>

{#if visible}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-md"
    style="pointer-events: none;"
  >
    <div class="text-center">
      <!-- Large upload icon with gentle animation -->
      <div class="mb-6 flex justify-center">
        <Upload
          class="text-primary h-24 w-24 animate-pulse"
          stroke-width="1.5"
        />
      </div>

      <!-- Main text -->
      <div class="mb-3 text-4xl font-bold text-white drop-shadow-lg">
        Drop your music here
      </div>

      <!-- Subtitle -->
      <div class="text-lg text-white/80 drop-shadow-md">
        Supports MP3, FLAC, and WAV files
      </div>
    </div>
  </div>
{/if}
