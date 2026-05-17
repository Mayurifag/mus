<script lang="ts">
  import { Slider } from "$lib/components/ui/slider";
  import type { AudioService } from "$lib/services/AudioService";
  import type { TimeRange } from "$lib/types";
  import { formatDuration } from "$lib/utils/formatters";

  let {
    audioService,
    progressValue = $bindable(0),
    currentTime,
    duration,
    bufferedRanges,
    disabled,
    wrapperClass,
  }: {
    audioService?: AudioService;
    progressValue: number;
    currentTime: number;
    duration: number;
    bufferedRanges: TimeRange[];
    disabled: boolean;
    wrapperClass: string;
  } = $props();

  function handleProgressCommit(): void {
    audioService?.endSeeking(progressValue);
  }
</script>

<div class="flex w-full items-center space-x-2 {wrapperClass}">
  <span class="text-muted-foreground w-10 text-right text-xs">
    {formatDuration(currentTime)}
  </span>
  <Slider
    bind:value={progressValue}
    onValueCommit={handleProgressCommit}
    onpointerdown={() => audioService?.startSeeking()}
    onValueChange={(v: number[]) => audioService?.seek(v[0])}
    max={duration || 100}
    step={1}
    class="flex-1 cursor-pointer"
    {disabled}
    {bufferedRanges}
  />
  <span class="text-muted-foreground w-10 text-xs">
    {formatDuration(duration)}
  </span>
</div>
