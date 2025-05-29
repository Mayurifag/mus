import type { Track } from "$lib/types";
import { getStreamUrl } from "$lib/services/apiClient";
import { writable } from "svelte/store";

export class AudioService {
  private audio: HTMLAudioElement;
  private onPlaybackFinishedCallback: () => void;
  private shouldAutoPlay = false;
  private lastAudioProgressSyncTime = 0;

  // Convert internal state to reactive stores
  private _volume = writable(1.0);
  private _isMuted = writable(false);
  private _isPlaying = writable(false);
  private _currentTime = writable(0);
  private _duration = writable(0);
  private _isRepeat = writable(false);

  // Keep current values for efficient getter access
  private _currentVolume = 1.0;
  private _currentIsMuted = false;
  private _currentIsPlaying = false;
  private _currentCurrentTime = 0;
  private _currentDuration = 0;
  private _currentIsRepeat = false;

  constructor(
    audio: HTMLAudioElement,
    onPlaybackFinishedWithoutRepeat: () => void,
  ) {
    this.audio = audio;
    this.onPlaybackFinishedCallback = onPlaybackFinishedWithoutRepeat;

    // Subscribe to our own stores to keep current values in sync
    this._volume.subscribe((value) => (this._currentVolume = value));
    this._isMuted.subscribe((value) => (this._currentIsMuted = value));
    this._isPlaying.subscribe((value) => (this._currentIsPlaying = value));
    this._currentTime.subscribe((value) => (this._currentCurrentTime = value));
    this._duration.subscribe((value) => (this._currentDuration = value));
    this._isRepeat.subscribe((value) => (this._currentIsRepeat = value));

    this.setupEventListeners();
  }

  private setupEventListeners(): void {
    this.audio.addEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.addEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.addEventListener("ended", this.handleEnded);
    this.audio.addEventListener("error", this.handleError);
    this.audio.addEventListener("canplay", this.handleCanPlay);
    this.audio.addEventListener("play", this.handlePlay);
    this.audio.addEventListener("pause", this.handlePause);
  }

  private handlePlay = (): void => {
    this._isPlaying.set(true);
  };

  private handlePause = (): void => {
    this._isPlaying.set(false);
  };

  private handleLoadedMetadata = (): void => {
    if (this.audio && !isNaN(this.audio.duration)) {
      this._duration.set(this.audio.duration);
    }
  };

  private handleTimeUpdate = (): void => {
    if (this.audio && !isNaN(this.audio.currentTime)) {
      this._currentTime.set(this.audio.currentTime);
    }
  };

  private handleEnded = (): void => {
    if (this._currentIsRepeat) {
      if (this.audio) {
        this.audio.currentTime = 0;
        this.audio.play().catch((error) => {
          console.error("Error replaying audio:", error);
          this.pause();
        });
      }
    } else {
      this.onPlaybackFinishedCallback();
    }
  };

  private handleError = (): void => {
    console.error("Audio playback error occurred");
    this.pause();
  };

  private handleCanPlay = (): void => {
    if (this.shouldAutoPlay) {
      this.audio.play().catch((error) => {
        console.error("Error auto-playing audio:", error);
        this.pause();
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
      if (typeof document !== "undefined") {
        document.title = `${track.artist} - ${track.title}`;
      }
      this._currentTime.set(0);
    }
  }

  play(): void {
    this.audio.play().catch((error) => {
      console.error("Error playing audio:", error);
      if (error.name !== "AbortError") {
        this.pause();
      }
    });
  }

  pause(): void {
    this.audio.pause();
  }

  setVolume(volume: number): void {
    const newVolume = Math.max(0, Math.min(1, volume));
    this._volume.set(newVolume);
    this.audio.volume = this._currentIsMuted ? 0 : newVolume;
  }

  toggleMute(): void {
    this._isMuted.update((muted) => {
      const newMuted = !muted;
      this.audio.volume = newMuted ? 0 : this._currentVolume;
      return newMuted;
    });
  }

  setCurrentTime(time: number): void {
    const now = Date.now();
    const timeDiff = Math.abs(this.audio.currentTime - time);
    if (timeDiff > 1 && now - this.lastAudioProgressSyncTime > 100) {
      this._currentTime.set(time);
      this.audio.currentTime = time;
      this.lastAudioProgressSyncTime = now;
    }
  }

  get volume(): number {
    return this._currentVolume;
  }

  get isMuted(): boolean {
    return this._currentIsMuted;
  }

  get isPlaying(): boolean {
    return this._currentIsPlaying;
  }

  get currentTime(): number {
    return this._currentCurrentTime;
  }

  get duration(): number {
    return this._currentDuration;
  }

  get isRepeat(): boolean {
    return this._currentIsRepeat;
  }

  // Store getters for reactive subscriptions
  get volumeStore() {
    return this._volume;
  }

  get isMutedStore() {
    return this._isMuted;
  }

  get isPlayingStore() {
    return this._isPlaying;
  }

  get currentTimeStore() {
    return this._currentTime;
  }

  get durationStore() {
    return this._duration;
  }

  get isRepeatStore() {
    return this._isRepeat;
  }

  setRepeat(isRepeat: boolean): void {
    this._isRepeat.set(isRepeat);
  }

  toggleRepeat(): void {
    this._isRepeat.update((current) => !current);
  }

  initializeState(volume: number, isMuted: boolean, isRepeat?: boolean): void {
    this._volume.set(volume);
    this._isMuted.set(isMuted);
    this.audio.volume = isMuted ? 0 : volume;
    if (isRepeat !== undefined) {
      this._isRepeat.set(isRepeat);
    }
  }

  destroy(): void {
    this.audio.removeEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.removeEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.removeEventListener("ended", this.handleEnded);
    this.audio.removeEventListener("error", this.handleError);
    this.audio.removeEventListener("canplay", this.handleCanPlay);
    this.audio.removeEventListener("play", this.handlePlay);
    this.audio.removeEventListener("pause", this.handlePause);
  }
}
