<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { Slider } from "$lib/components/ui/slider";
  import type { AudioService } from "$lib/services/AudioService";
  import { Volume2, VolumeX } from "@lucide/svelte";

  let {
    audioService,
    isMuted,
    volumeValue = $bindable(1),
    wrapperClass,
    buttonClass,
    iconClass,
  }: {
    audioService?: AudioService;
    isMuted: boolean;
    volumeValue: number;
    wrapperClass: string;
    buttonClass: string;
    iconClass: string;
  } = $props();

  let isVolumeHovered = $state(false);
  let volumeFeedbackValue = $derived(
    isMuted ? 0 : Math.round(volumeValue * 100),
  );

  function handleVolumeChange(value: number): void {
    audioService?.setVolume(value);
  }
</script>

<Button
  variant="ghost"
  size="icon"
  class="icon-glow-effect {buttonClass}"
  onclick={() => audioService?.toggleMute()}
  aria-label={isMuted ? "Unmute" : "Mute"}
>
  {#if isMuted}
    <VolumeX class={iconClass} />
  {:else}
    <Volume2 class={iconClass} />
  {/if}
</Button>

<div
  class="relative cursor-pointer {wrapperClass}"
  role="slider"
  aria-label="Volume control"
  aria-valuenow={volumeFeedbackValue}
  aria-valuemin="0"
  aria-valuemax="100"
  tabindex="0"
  onmouseenter={() => (isVolumeHovered = true)}
  onmouseleave={() => (isVolumeHovered = false)}
>
  <Slider
    bind:value={volumeValue}
    onValueChange={(v: number[]) => handleVolumeChange(v[0])}
    max={1}
    step={0.01}
    class="w-full"
  />
  {#if isVolumeHovered}
    <div
      class="bg-muted absolute -top-7 left-1/2 -translate-x-1/2 rounded px-2 py-1 text-xs font-medium text-white transition-opacity"
    >
      {volumeFeedbackValue}%
    </div>
  {/if}
</div>
