<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { trackStore } from "$lib/stores/trackStore";
  import type { AudioService } from "$lib/services/AudioService";
  import {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Shuffle,
    Repeat,
    Repeat1,
  } from "@lucide/svelte";

  let {
    audioService,
    isPlaying,
    isRepeat,
    buttonClass,
    playButtonClass,
    iconClass,
    playIconClass,
    repeatSecond = false,
  }: {
    audioService?: AudioService;
    isPlaying: boolean;
    isRepeat: boolean;
    buttonClass: string;
    playButtonClass: string;
    iconClass: string;
    playIconClass: string;
    repeatSecond?: boolean;
  } = $props();

  function togglePlay(): void {
    if (!audioService) return;

    if (isPlaying) {
      audioService.pause();
    } else {
      audioService.play();
    }
  }
</script>

<Button
  variant="ghost"
  size="icon"
  class="icon-glow-effect {buttonClass} {$trackStore.is_shuffle
    ? 'bg-accent/10'
    : ''}"
  onclick={() => trackStore.toggleShuffle()}
  aria-label="Toggle Shuffle"
  aria-pressed={$trackStore.is_shuffle}
>
  <Shuffle
    class={iconClass}
    color={$trackStore.is_shuffle ? "hsl(var(--accent))" : "currentColor"}
  />
</Button>

{#if repeatSecond}
  <Button
    variant="ghost"
    size="icon"
    class="icon-glow-effect {buttonClass} {isRepeat ? 'bg-accent/10' : ''}"
    onclick={() => audioService?.toggleRepeat()}
    aria-label="Toggle Repeat"
    aria-pressed={isRepeat}
  >
    {#if isRepeat}
      <Repeat1 class={iconClass} color="hsl(var(--accent))" />
    {:else}
      <Repeat class={iconClass} color="currentColor" />
    {/if}
  </Button>
{/if}

<Button
  variant="ghost"
  size="icon"
  class="icon-glow-effect {buttonClass}"
  onclick={() => trackStore.previousTrack()}
  aria-label="Previous Track"
  disabled={!$trackStore.currentTrack}
>
  <SkipBack class={iconClass} />
</Button>

<Button
  variant="ghost"
  size="icon"
  class="icon-glow-effect {playButtonClass}"
  onclick={togglePlay}
  aria-label={isPlaying ? "Pause" : "Play"}
  disabled={!$trackStore.currentTrack}
>
  {#if isPlaying}
    <Pause class={playIconClass} />
  {:else}
    <Play class={playIconClass} />
  {/if}
</Button>

<Button
  variant="ghost"
  size="icon"
  class="icon-glow-effect {buttonClass}"
  onclick={() => trackStore.nextTrack()}
  aria-label="Next Track"
  disabled={!$trackStore.currentTrack}
>
  <SkipForward class={iconClass} />
</Button>

{#if !repeatSecond}
  <Button
    variant="ghost"
    size="icon"
    class="icon-glow-effect {buttonClass} {isRepeat ? 'bg-accent/10' : ''}"
    onclick={() => audioService?.toggleRepeat()}
    aria-label="Toggle Repeat"
    aria-pressed={isRepeat}
  >
    {#if isRepeat}
      <Repeat1 class={iconClass} color="hsl(var(--accent))" />
    {:else}
      <Repeat class={iconClass} color="currentColor" />
    {/if}
  </Button>
{/if}
