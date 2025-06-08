import type { Track, TimeRange } from "$lib/types";
import { getStreamUrl } from "$lib/services/apiClient";
import { writable } from "svelte/store";
import { trackStore } from "$lib/stores/trackStore";

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
  private _currentBufferedRanges = writable<TimeRange[]>([]);

  // Keep current values for efficient getter access
  private _currentVolume = 1.0;
  private _currentIsMuted = false;
  private _currentIsPlaying = false;
  private _currentCurrentTime = 0;
  private _currentDuration = 0;
  private _currentIsRepeat = false;
  private _currentCurrentBufferedRanges: TimeRange[] = [];

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
    this._currentBufferedRanges.subscribe(
      (value) => (this._currentCurrentBufferedRanges = value),
    );

    this.setupEventListeners();
    this.setupMediaSession();
  }

  private setupEventListeners(): void {
    this.audio.addEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.addEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.addEventListener("ended", this.handleEnded);
    this.audio.addEventListener("error", this.handleError);
    this.audio.addEventListener("canplay", this.handleCanPlay);
    this.audio.addEventListener("play", this.handlePlay);
    this.audio.addEventListener("pause", this.handlePause);
    this.audio.addEventListener("progress", this.handleProgress);
    this.audio.addEventListener("suspend", this.handleSuspend);
  }

  private handlePlay = (): void => {
    this._isPlaying.set(true);
    this.updateMediaSessionPlaybackState();
  };

  private handlePause = (): void => {
    this._isPlaying.set(false);
    this.updateMediaSessionPlaybackState();
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

  private handleProgress = (): void => {
    this.updateBufferedRanges();
  };

  private handleSuspend = (): void => {
    this.updateBufferedRanges();
  };

  private convertTimeRangesToArray(timeRanges: TimeRanges): TimeRange[] {
    const ranges: TimeRange[] = [];
    for (let i = 0; i < timeRanges.length; i++) {
      ranges.push({
        start: timeRanges.start(i),
        end: timeRanges.end(i),
      });
    }
    return ranges;
  }

  private updateBufferedRanges(): void {
    if (this.audio && this.audio.buffered) {
      const ranges = this.convertTimeRangesToArray(this.audio.buffered);
      this._currentBufferedRanges.set(ranges);
    }
  }

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
      this._currentBufferedRanges.set([]);
      this.updateMediaSessionMetadata(track);
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

  get currentBufferedRanges(): TimeRange[] {
    return this._currentCurrentBufferedRanges;
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

  get currentBufferedRangesStore() {
    return this._currentBufferedRanges;
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

  private setupMediaSession(): void {
    if (!("mediaSession" in navigator)) return;

    navigator.mediaSession.setActionHandler("play", () => {
      this.play();
    });

    navigator.mediaSession.setActionHandler("pause", () => {
      this.pause();
    });

    navigator.mediaSession.setActionHandler("nexttrack", () => {
      trackStore.nextTrack();
    });

    navigator.mediaSession.setActionHandler("previoustrack", () => {
      trackStore.previousTrack();
    });

    navigator.mediaSession.setActionHandler("seekto", (details) => {
      if (details.seekTime !== undefined) {
        this.setCurrentTime(details.seekTime);
      }
    });
  }

  private updateMediaSessionMetadata(track: Track): void {
    if (!("mediaSession" in navigator)) return;

    const artwork: MediaImage[] = [];
    if (track.cover_original_url) {
      artwork.push({
        src: track.cover_original_url,
        sizes: "512x512",
        type: "image/webp",
      });
    }

    const metadata: MediaMetadataInit = {
      title: track.title,
      artist: track.artist,
    };

    if (artwork.length > 0) {
      metadata.artwork = artwork;
    }

    navigator.mediaSession.metadata = new MediaMetadata(metadata);
  }

  private updateMediaSessionPlaybackState(): void {
    if (!("mediaSession" in navigator)) return;
    navigator.mediaSession.playbackState = this._currentIsPlaying
      ? "playing"
      : "paused";
  }

  destroy(): void {
    this.audio.removeEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.removeEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.removeEventListener("ended", this.handleEnded);
    this.audio.removeEventListener("error", this.handleError);
    this.audio.removeEventListener("canplay", this.handleCanPlay);
    this.audio.removeEventListener("play", this.handlePlay);
    this.audio.removeEventListener("pause", this.handlePause);
    this.audio.removeEventListener("progress", this.handleProgress);
    this.audio.removeEventListener("suspend", this.handleSuspend);
  }
}
