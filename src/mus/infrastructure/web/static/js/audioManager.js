// Audio playback and management functionality
export const audioManager = {
  audioPlayer: null,
  preloadedTracks: new Map(),
  preloadSize: 128 * 1024, // Preload first 128KB of each track

  init (audioElement) {
    this.audioPlayer = audioElement
    this.preloadedTracks.clear()
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
  },

  async preloadTrack (trackId) {
    if (this.preloadedTracks.has(trackId)) {
      return
    }

    try {
      const response = await fetch(`/stream/${trackId}`, {
        headers: {
          Range: `bytes=0-${this.preloadSize - 1}`
        }
      })

      if (response.ok || response.status === 206) {
        const blob = await response.blob()
        this.preloadedTracks.set(trackId, blob)
      }
    } catch (error) {
      console.error(`Failed to preload track ${trackId}:`, error)
    }
  },

  clearPreloadedTrack (trackId) {
    this.preloadedTracks.delete(trackId)
  },

  clearOldPreloadedTracks (currentTrackId, nextTrackId, prevTrackId) {
    for (const trackId of this.preloadedTracks.keys()) {
      if (trackId !== currentTrackId &&
        trackId !== nextTrackId &&
        trackId !== prevTrackId) {
        this.preloadedTracks.delete(trackId)
      }
    }
  }
}
