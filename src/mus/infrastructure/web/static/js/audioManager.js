// Audio playback and management functionality
export const audioManager = {
  audioPlayer: null,

  init (audioElement) {
    this.audioPlayer = audioElement
  },

  play () {
    return this.audioPlayer.play()
  },

  pause () {
    this.audioPlayer.pause()
  },

  seek (position) {
    if (typeof position === 'number') {
      // If position is a number, treat it as a percentage of the duration
      this.audioPlayer.currentTime = position * this.audioPlayer.duration
    } else {
      // Otherwise, treat it as an absolute time in seconds
      this.audioPlayer.currentTime = position
    }
  },

  loadTrack (url) {
    this.audioPlayer.src = url
    this.audioPlayer.load()
  },

  getCurrentTime () {
    return this.audioPlayer.currentTime
  },

  getDuration () {
    return this.audioPlayer.duration
  },

  isPaused () {
    return this.audioPlayer.paused
  }
}
