import type { Track, TimeRange } from "$lib/types";
import { get, writable } from "svelte/store";
import { trackStore } from "$lib/stores/trackStore";
import { formatArtistsForDisplay } from "$lib/utils/formatters";
import Hls from "hls.js/light";
import {
  clearMediaSession,
  setupMediaSessionHandlers,
  updateMediaSessionMetadata,
  updateMediaSessionPlaybackState,
} from "$lib/services/mediaSessionService";
import { displayTrackTitle } from "$lib/utils/categoryMarkers";
import { toast } from "svelte-sonner";

export class AudioService {
  private audio: HTMLAudioElement;
  private isSeeking = false;
  private currentTrackId: string | null = null;
  private pendingHlsUrl: string | null = null;
  private hasRequestedLoad = false;
  private pendingInitialTime: number | null = null;
  private hls: Hls | null = null;
  private readonly initialTitle =
    typeof document === "undefined" ? "" : document.title;

  private _volume = writable(1.0);
  private _isMuted = writable(false);
  private _isPlaying = writable(false);
  private _currentTime = writable(0);
  private _duration = writable(0);
  private _isRepeat = writable(false);
  private _currentBufferedRanges = writable<TimeRange[]>([]);

  constructor(audio: HTMLAudioElement) {
    this.audio = audio;

    this.setupEventListeners();
  }

  // TODO: remove this.audio.controls if that wont work for ios PWA
  private setupEventListeners(): void {
    this.audio.controls = true;
    this.audio.addEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.addEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.addEventListener("ended", this.handleEnded);
    this.audio.addEventListener("error", this.handleError);
    this.audio.addEventListener("play", this.handlePlay);
    this.audio.addEventListener("pause", this.handlePause);
    this.audio.addEventListener("playing", this.setupActionHandlers);
    this.audio.addEventListener("progress", this.handleProgress);
    this.audio.addEventListener("suspend", this.handleSuspend);
  }

  private handlePlay = (): void => {
    if (this.currentTrackId === null) return;
    this._isPlaying.set(true);
    updateMediaSessionPlaybackState(true);
  };

  private handlePause = (): void => {
    this._isPlaying.set(false);
    if (this.currentTrackId !== null) updateMediaSessionPlaybackState(false);
  };

  private handleLoadedMetadata = (): void => {
    if (this.currentTrackId === null) return;
    if (this.pendingInitialTime !== null) {
      const time = this.pendingInitialTime;
      this.pendingInitialTime = null;
      this.setTime(time);
    }
    if (this.audio && !isNaN(this.audio.duration)) {
      this._duration.set(this.audio.duration);
    }
  };

  private handleTimeUpdate = (): void => {
    if (this.currentTrackId === null || this.isSeeking) return;

    if (this.audio && !isNaN(this.audio.currentTime)) {
      this._currentTime.set(this.audio.currentTime);
    }
  };

  private handleEnded = (): void => {
    if (this.currentTrackId === null) return;
    if (this.isRepeat) {
      if (this.audio) {
        this.audio.currentTime = 0;
        this.audio.play().catch((error) => {
          console.error("Error replaying audio:", error);
          this.pause();
        });
      }
    } else {
      trackStore.nextTrack(true);
    }
  };

  private handleError = (): void => {
    console.error("Audio playback error occurred");
    this.pause();
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
    if (this.currentTrackId === null) return;
    if (this.audio && this.audio.buffered) {
      const ranges = this.convertTimeRangesToArray(this.audio.buffered);
      this._currentBufferedRanges.set(ranges);
    }
  }

  private loadSource(): void {
    if (this.currentTrackId === null || this.hasRequestedLoad) return;

    if (this.pendingHlsUrl !== null) {
      this.loadHlsSource(this.pendingHlsUrl);
      this.pendingHlsUrl = null;
    }
    this.hasRequestedLoad = true;
  }

  private loadHlsSource(hlsUrl: string): void {
    this.destroyHls();
    if (Hls.isSupported()) {
      this.hls = new Hls();
      this.hls.loadSource(hlsUrl);
      this.hls.attachMedia(this.audio);
      this.hls.on(Hls.Events.ERROR, (_event, data) => {
        if (!data.fatal) return;
        console.error("HLS playback error occurred", data);
        this.pause();
      });
      return;
    }

    if (this.audio.canPlayType("application/vnd.apple.mpegurl")) {
      this.audio.src = hlsUrl;
      this.audio.load();
      return;
    }

    console.error("HLS playback is not supported by this browser");
  }

  private destroyHls(): void {
    this.hls?.destroy();
    this.hls = null;
  }

  private handlePlayError(error: DOMException): void {
    if (error.name === "AbortError") return;

    if (error.name === "NotAllowedError") {
      toast.info("Tap play to resume");
    } else {
      console.error("Error playing audio:", error);
    }
    this.pause();
  }

  updateAudioSource(
    track: Track | null,
    isPlaying: boolean,
    initialTime = 0,
  ): void {
    if (!track) {
      this.clearSource();
      return;
    }

    if (typeof document !== "undefined") {
      document.title = `${formatArtistsForDisplay(track.artist)} - ${displayTrackTitle(track)}`;
    }
    updateMediaSessionMetadata(track);

    const shouldUpdateSource = this.currentTrackId !== track.id;
    this.currentTrackId = track.id;
    if (shouldUpdateSource) {
      this.destroyHls();
      this.hasRequestedLoad = false;
      this.pendingHlsUrl = track.hls_url;
      this.pendingInitialTime = initialTime;
      this._currentTime.set(initialTime);
      this._duration.set(0);
      this._currentBufferedRanges.set([]);

      if (isPlaying) {
        this.loadSource();
        this.audio.play().catch((error) => this.handlePlayError(error));
      }
    } else if (initialTime > 0) {
      this.setTime(initialTime);
    }
  }

  play(): void {
    if (this.currentTrackId === null) return;
    this.loadSource();
    this.audio.play().catch((error) => this.handlePlayError(error));
  }

  pause(): void {
    this.audio.pause();
  }

  setVolume(volume: number): void {
    const newVolume = Math.max(0, Math.min(1, volume));
    this._volume.set(newVolume);
    this.audio.volume = this.isMuted ? 0 : newVolume;
  }

  toggleMute(): void {
    this._isMuted.update((muted) => {
      const newMuted = !muted;
      this.audio.volume = newMuted ? 0 : this.volume;
      return newMuted;
    });
  }

  setTime(time: number): void {
    if (this.pendingInitialTime !== null) this.pendingInitialTime = time;
    this._currentTime.set(time);
    this.loadSource();
    this.audio.currentTime = time;
  }

  startSeeking(): void {
    this.isSeeking = true;
  }

  seek(time: number): void {
    this._currentTime.set(time);
  }

  endSeeking(time: number): void {
    this.setTime(time);
    this.isSeeking = false;
  }

  get volume(): number {
    return get(this._volume);
  }

  get isMuted(): boolean {
    return get(this._isMuted);
  }

  get isPlaying(): boolean {
    return get(this._isPlaying);
  }

  get currentTime(): number {
    return get(this._currentTime);
  }

  get duration(): number {
    return get(this._duration);
  }

  get isRepeat(): boolean {
    return get(this._isRepeat);
  }

  get currentBufferedRanges(): TimeRange[] {
    return get(this._currentBufferedRanges);
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

  private setupActionHandlers = (): void => {
    if (this.currentTrackId === null) return;
    setupMediaSessionHandlers({
      play: () => this.play(),
      pause: () => this.pause(),
      nextTrack: () => trackStore.nextTrack(),
      previousTrack: () => trackStore.previousTrack(),
      seekTo: (time) => this.setTime(time),
    });
  };

  private clearSource(): void {
    this.currentTrackId = null;
    this.pendingHlsUrl = null;
    this.pendingInitialTime = null;
    this.hasRequestedLoad = false;
    this.isSeeking = false;
    this.pause();
    this.destroyHls();
    if (typeof this.audio.removeAttribute === "function") {
      this.audio.removeAttribute("src");
    } else {
      this.audio.src = "";
    }
    this.audio.load();
    this._isPlaying.set(false);
    this._currentTime.set(0);
    this._duration.set(0);
    this._currentBufferedRanges.set([]);
    clearMediaSession();
    if (typeof document !== "undefined") document.title = this.initialTitle;
  }

  destroy(): void {
    this.clearSource();
    this.audio.controls = false;
    this.audio.removeEventListener("loadedmetadata", this.handleLoadedMetadata);
    this.audio.removeEventListener("timeupdate", this.handleTimeUpdate);
    this.audio.removeEventListener("ended", this.handleEnded);
    this.audio.removeEventListener("error", this.handleError);
    this.audio.removeEventListener("play", this.handlePlay);
    this.audio.removeEventListener("pause", this.handlePause);
    this.audio.removeEventListener("playing", this.setupActionHandlers);
    this.audio.removeEventListener("progress", this.handleProgress);
    this.audio.removeEventListener("suspend", this.handleSuspend);
  }
}
