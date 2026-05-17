import type { AudioService } from "$lib/services/AudioService";
import type { TimeRange } from "$lib/types";

interface PlayerFooterHandlers {
  setIsPlaying: (value: boolean) => void;
  setIsMuted: (value: boolean) => void;
  setCurrentTime: (value: number) => void;
  setDuration: (value: number) => void;
  setVolume: (value: number) => void;
  setIsRepeat: (value: boolean) => void;
  setBufferedRanges: (value: TimeRange[]) => void;
}

export function subscribeToPlayerFooterStores(
  audioService: AudioService,
  handlers: PlayerFooterHandlers,
): () => void {
  const unsubscribers: (() => void)[] = [];

  if (audioService.isPlayingStore) {
    unsubscribers.push(
      audioService.isPlayingStore.subscribe(handlers.setIsPlaying),
    );
  }
  if (audioService.isMutedStore) {
    unsubscribers.push(
      audioService.isMutedStore.subscribe(handlers.setIsMuted),
    );
  }
  if (audioService.currentTimeStore) {
    unsubscribers.push(
      audioService.currentTimeStore.subscribe(handlers.setCurrentTime),
    );
  }
  if (audioService.durationStore) {
    unsubscribers.push(
      audioService.durationStore.subscribe(handlers.setDuration),
    );
  }
  if (audioService.volumeStore) {
    unsubscribers.push(audioService.volumeStore.subscribe(handlers.setVolume));
  }
  if (audioService.isRepeatStore) {
    unsubscribers.push(
      audioService.isRepeatStore.subscribe(handlers.setIsRepeat),
    );
  }
  if (audioService.currentBufferedRangesStore) {
    unsubscribers.push(
      audioService.currentBufferedRangesStore.subscribe(
        handlers.setBufferedRanges,
      ),
    );
  }

  return () => unsubscribers.forEach((unsubscribe) => unsubscribe());
}
