// Audio playback and management functionality
import { mediaSessionManager } from './mediaSessionManager.js'
import { trackManager } from './trackManager.js'

export const audioManager = {
  audioPlayer: null,
  preloadedTracks: new Map(),
  preloadSize: 128 * 1024, // Preload first 128KB of each track

  init (audioElement) {
    this.audioPlayer = audioElement
    this.preloadedTracks.clear()

    // Set up media session handlers
    mediaSessionManager.setActionHandlers({
      onPlay: () => this.play(),
      onPause: () => this.pause(),
      onNextTrack: () => {
        const nextIndex = trackManager.getNextTrack()
        if (nextIndex !== -1) {
          trackManager.playTrackAtIndex(nextIndex)
        }
      },
      onPreviousTrack: () => {
        const prevIndex = trackManager.getPreviousTrack()
        if (prevIndex !== -1) {
          trackManager.playTrackAtIndex(prevIndex)
        }
      },
      onSeekTo: (time) => this.seek(time / this.getDuration()),
      getPositionState: () => ({
        duration: this.getDuration() || 0,
        position: this.getCurrentTime() || 0,
        playbackRate: this.audioPlayer.playbackRate
      })
    })
  },

  play () {
    const playPromise = this.audioPlayer.play()
    if (playPromise) {
      playPromise.then(() => {
        mediaSessionManager.updatePlaybackState('playing')
      })
    }
    return playPromise
  },

  pause () {
    this.audioPlayer.pause()
    mediaSessionManager.updatePlaybackState('paused')
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
