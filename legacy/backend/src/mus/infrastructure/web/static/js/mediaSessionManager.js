// Media Session API integration
const SEEK_SKIP_SECONDS = 10

export const mediaSessionManager = {
  isSupported: 'mediaSession' in navigator,

  updateMetadata (title, artist, artworkUrl) {
    if (!this.isSupported) return

    const artworkType = artworkUrl.endsWith('.webp') ? 'image/webp' : 'image/png'
    const artworkSize = artworkUrl.endsWith('.webp') ? '128x128' : '512x512'

    navigator.mediaSession.metadata = new MediaMetadata({
      title,
      artist,
      artwork: [
        {
          src: artworkUrl,
          sizes: artworkSize,
          type: artworkType
        }
      ]
    })
  },

  updatePlaybackState (state) {
    if (!this.isSupported) return
    navigator.mediaSession.playbackState = state
  },

  setActionHandlers ({
    onPlay,
    onPause,
    onNextTrack,
    onPreviousTrack,
    onSeekBackward,
    onSeekForward,
    onSeekTo,
    getPositionState
  }) {
    if (!this.isSupported) return

    // Set up action handlers
    navigator.mediaSession.setActionHandler('play', onPlay)
    navigator.mediaSession.setActionHandler('pause', onPause)
    navigator.mediaSession.setActionHandler('nexttrack', onNextTrack)
    navigator.mediaSession.setActionHandler('previoustrack', onPreviousTrack)

    // Seek handlers
    navigator.mediaSession.setActionHandler('seekbackward', () => {
      const currentTime = getPositionState().position
      onSeekTo(Math.max(0, currentTime - SEEK_SKIP_SECONDS))
    })

    navigator.mediaSession.setActionHandler('seekforward', () => {
      const currentTime = getPositionState().position
      const duration = getPositionState().duration
      onSeekTo(Math.min(duration, currentTime + SEEK_SKIP_SECONDS))
    })

    navigator.mediaSession.setActionHandler('seekto', (event) => {
      onSeekTo(event.seekTime)
    })

    // Update position state periodically
    setInterval(() => {
      if (this.isSupported && getPositionState) {
        navigator.mediaSession.setPositionState(getPositionState())
      }
    }, 1000)
  }
}
