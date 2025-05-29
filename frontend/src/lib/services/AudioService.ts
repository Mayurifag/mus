import type { Track } from "$lib/types";
import { get, type Readable } from "svelte/store";
import { getStreamUrl } from "$lib/services/apiClient";

interface PlayerStoreState {
  currentTrack: Track | null;
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  isMuted: boolean;
  is_shuffle: boolean;
  is_repeat: boolean;
}

interface PlayerStoreType extends Readable<PlayerStoreState> {
  setCurrentTime: (time: number) => void;
  setDuration: (duration: number) => void;
  pause: () => void;
}

interface TrackStoreType {
  nextTrack: () => void;
}

export class AudioService {
  private audio: HTMLAudioElement;
  private playerStore: PlayerStoreType;
  private trackStore: TrackStoreType;
  private shouldAutoPlay = false;
  private lastAudioProgressSyncTime = 0;

  constructor(
    audio: HTMLAudioElement,
    playerStore: PlayerStoreType,
    trackStore: TrackStoreType,
  ) {
    this.audio = audio;
    this.playerStore = playerStore;
    this.trackStore = trackStore;
    this.setupEventListeners();
  }

  private setupEventListeners(): void {
    this.audio.addEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.addEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.addEventListener("ended", this.handleEnded);
    this.audio.addEventListener("error", this.handleError);
    this.audio.addEventListener("canplay", this.handleCanPlay);
  }

  private handleLoadedMetadata = (): void => {
    if (this.audio && !isNaN(this.audio.duration)) {
      this.playerStore.setDuration(this.audio.duration);
    }
  };

  private handleTimeUpdate = (): void => {
    if (this.audio && !isNaN(this.audio.currentTime)) {
      this.playerStore.setCurrentTime(this.audio.currentTime);
    }
  };

  private handleEnded = (): void => {
    const playerState = get(this.playerStore);
    if (playerState.is_repeat) {
      if (this.audio) {
        this.audio.currentTime = 0;
        this.audio.play().catch((error) => {
          console.error("Error replaying audio:", error);
          this.playerStore.pause();
        });
      }
    } else {
      this.trackStore.nextTrack();
    }
  };

  private handleError = (): void => {
    console.error("Audio playback error occurred");
    this.playerStore.pause();
  };

  private handleCanPlay = (): void => {
    if (this.shouldAutoPlay) {
      this.audio.play().catch((error) => {
        console.error("Error auto-playing audio:", error);
        this.playerStore.pause();
      });
      this.shouldAutoPlay = false;
    }
  };

  updateAudioSource(track: Track | null, isPlaying: boolean): void {
    if (!track) return;

    const streamUrl = getStreamUrl(track.id);
    if (this.audio.src !== streamUrl) {
      this.shouldAutoPlay = isPlaying;
      this.audio.src = streamUrl;
      this.audio.load();
    }
  }

  play(): void {
    this.audio.play().catch((error) => {
      console.error("Error playing audio:", error);
      if (error.name !== "AbortError") {
        this.playerStore.pause();
      }
    });
  }

  pause(): void {
    this.audio.pause();
  }

  setVolume(volume: number, isMuted: boolean): void {
    this.audio.volume = isMuted ? 0 : volume;
  }

  setCurrentTime(time: number): void {
    const now = Date.now();
    const timeDiff = Math.abs(this.audio.currentTime - time);
    if (timeDiff > 1 && now - this.lastAudioProgressSyncTime > 100) {
      this.audio.currentTime = time;
      this.lastAudioProgressSyncTime = now;
    }
  }

  destroy(): void {
    this.audio.removeEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.removeEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.removeEventListener("ended", this.handleEnded);
    this.audio.removeEventListener("error", this.handleError);
    this.audio.removeEventListener("canplay", this.handleCanPlay);
  }
}
