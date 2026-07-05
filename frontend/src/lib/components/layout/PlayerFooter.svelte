<script lang="ts">
  import { Card } from "$lib/components/ui/card";
  import { Button } from "$lib/components/ui/button";
  import { trackStore } from "$lib/stores/trackStore";
  import type { AudioService } from "$lib/services/AudioService";
  import type { TimeRange } from "$lib/types";
  import { updateEffectStats } from "$lib/utils/monitoredEffect";
  import { Menu } from "@lucide/svelte";
  import { browser } from "$app/environment";
  import CurrentTrackInfo from "$lib/components/layout/player/CurrentTrackInfo.svelte";
  import PlaybackControls from "$lib/components/layout/player/PlaybackControls.svelte";
  import ProgressControl from "$lib/components/layout/player/ProgressControl.svelte";
  import VolumeControl from "$lib/components/layout/player/VolumeControl.svelte";
  import { subscribeToPlayerFooterStores } from "$lib/components/layout/player/playerFooterSubscriptions";

  let { audioService }: { audioService?: AudioService } = $props();
  let progressValue = $state(0);
  let volumeValue = $state(1);
  let isPlaying = $state(false);
  let isMuted = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let isRepeat = $state(false);
  let bufferedRanges = $state<TimeRange[]>([]);

  $effect(() => {
    updateEffectStats("PlayerFooter_SubscriptionManager");
    if (audioService) {
      return subscribeToPlayerFooterStores(audioService, {
        setIsPlaying: (value) => {
          isPlaying = value;
        },
        setIsMuted: (value) => {
          isMuted = value;
        },
        setCurrentTime: (value) => {
          currentTime = value;
          progressValue = value;
        },
        setDuration: (value) => {
          duration = value;
        },
        setVolume: (value) => {
          volumeValue = value;
        },
        setIsRepeat: (value) => {
          isRepeat = value;
        },
        setBufferedRanges: (value) => {
          bufferedRanges = value;
        },
      });
    }
  });

  function toggleMenu() {
    if (browser) {
      const event = new CustomEvent("toggle-sheet");
      document.body.dispatchEvent(event);
    }
  }
</script>

<div
  class="bg-overlay sm700:h-[var(--footer-height-desktop)] fixed right-0 bottom-0 left-0 z-50 h-[var(--footer-height-mobile)] border-t"
  style="padding-bottom: var(--safe-area-inset-bottom); padding-left: var(--safe-area-inset-left); padding-right: var(--safe-area-inset-right);"
>
  <Card class="bg-overlay h-full rounded-none border-0 shadow-none">
    <div
      class="sm700:hidden flex h-[var(--footer-height-mobile)] flex-col gap-2 p-3"
    >
      <div class="flex w-full items-center justify-center gap-4">
        <PlaybackControls
          {audioService}
          {isPlaying}
          {isRepeat}
          buttonClass="h-12 w-12"
          playButtonClass="h-14 w-14"
          iconClass="h-6 w-6"
          playIconClass="h-7 w-7"
        />
      </div>

      <div class="flex w-full items-center justify-center gap-3">
        <VolumeControl
          {audioService}
          {isMuted}
          bind:volumeValue
          wrapperClass="w-32 flex-shrink-0"
          buttonClass="h-12 w-12 flex-shrink-0"
          iconClass="h-6 w-6"
        />
      </div>

      <ProgressControl
        {audioService}
        bind:progressValue
        {currentTime}
        {duration}
        {bufferedRanges}
        disabled={!$trackStore.currentTrack}
        wrapperClass="gap-3 px-4"
      />

      <CurrentTrackInfo track={$trackStore.currentTrack} mobile />
      <div class="flex-1"></div>
    </div>

    <div class="sm700:block relative hidden h-[var(--footer-height-desktop)]">
      <div
        class="desktop:right-[calc(50%+14rem+1.5rem)] absolute inset-y-0 right-[calc(50%+11rem+0.5rem)] left-0 flex min-w-0 items-center"
      >
        <CurrentTrackInfo track={$trackStore.currentTrack} />
      </div>

      <div
        class="desktop:w-[28rem] desktop:justify-center desktop:py-0 sm700:w-[22rem] absolute top-0 left-1/2 flex h-full -translate-x-1/2 flex-col items-center justify-around py-1"
      >
        <div
          class="desktop:space-x-2 flex w-full items-center justify-center space-x-2"
        >
          <PlaybackControls
            {audioService}
            {isPlaying}
            {isRepeat}
            buttonClass="h-9 w-9"
            playButtonClass="h-12 w-12"
            iconClass="h-5 w-5"
            playIconClass="h-7 w-7"
            repeatSecond
          />
          <VolumeControl
            {audioService}
            {isMuted}
            bind:volumeValue
            wrapperClass="w-24"
            buttonClass="h-9 w-9"
            iconClass="h-5 w-5"
          />
        </div>

        <ProgressControl
          {audioService}
          bind:progressValue
          {currentTime}
          {duration}
          {bufferedRanges}
          disabled={!$trackStore.currentTrack}
          wrapperClass="desktop:mt-2 mb-2 w-full"
        />
      </div>

      <div
        class="absolute inset-y-0 right-0 flex w-auto items-center justify-end space-x-2 pr-4"
      >
        <Button
          variant="ghost"
          size="icon"
          class="icon-glow-effect desktop:hidden ml-2"
          onclick={toggleMenu}
          aria-label="Open menu"
        >
          <Menu class="h-5 w-5" />
        </Button>
      </div>
    </div>
  </Card>
</div>
