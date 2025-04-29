// Audio playback and management functionality
export const audioManager = {
  audioPlayer: null,
  currentVolume: 1.0,
  isMuted: false,

  init(audioElement) {
    this.audioPlayer = audioElement;
    this.audioPlayer.volume = this.currentVolume;
  },

  play() {
    return this.audioPlayer.play();
  },

  pause() {
    this.audioPlayer.pause();
  },

  setVolume(volume) {
    this.currentVolume = volume;
    this.audioPlayer.volume = volume;
    this.isMuted = volume === 0;
    return this.isMuted;
  },

  toggleMute() {
    this.isMuted = !this.isMuted;
    this.audioPlayer.muted = this.isMuted;
    return this.isMuted;
  },

  seek(position) {
    this.audioPlayer.currentTime = position * this.audioPlayer.duration;
  },

  loadTrack(url) {
    this.audioPlayer.src = url;
    this.audioPlayer.load();
  },

  getCurrentTime() {
    return this.audioPlayer.currentTime;
  },

  getDuration() {
    return this.audioPlayer.duration;
  },

  isPaused() {
    return this.audioPlayer.paused;
  }
};
