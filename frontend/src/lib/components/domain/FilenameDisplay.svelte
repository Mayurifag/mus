<script lang="ts">
  import { formatDuration } from "$lib/utils/formatters";

  let {
    filename,
    fileSize,
    duration,
  }: {
    filename: string;
    fileSize?: number;
    duration?: number;
  } = $props();

  function formatFileSize(bytes: number): string {
    if (bytes === 0) return "0 Bytes";
    const k = 1024;
    const sizes = ["Bytes", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  }
</script>

<div class="bg-muted space-y-2 rounded-lg p-3">
  <div class="flex items-center justify-between">
    <div class="text-sm font-medium">Filename</div>
    {#if fileSize !== undefined}
      <div class="text-muted-foreground text-xs">
        {formatFileSize(fileSize)}
      </div>
    {/if}
  </div>
  <div class="flex items-center justify-between">
    <div class="text-muted-foreground font-mono text-sm">
      {filename}
    </div>
    {#if duration !== undefined}
      <div class="text-muted-foreground text-xs">
        {formatDuration(duration)}
      </div>
    {/if}
  </div>
</div>
