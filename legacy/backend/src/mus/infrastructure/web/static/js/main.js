import { audioManager } from './audioManager.js'
import { trackManager } from './trackManager.js'
import { uiControls } from './uiControls.js'
import { stateManager } from './stateManager.js'
import { volumeManager } from './volume.js'
// import { mediaSessionManager } from './mediaSessionManager.js'

document.addEventListener('DOMContentLoaded', () => {
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/static/service-worker.js')
      .then(registration => {
        console.log('ServiceWorker registration successful')
      })
      .catch(error => {
        console.error('ServiceWorker registration failed:', error)
      })
  }

  // Load initial state
  const initialState = stateManager.loadInitialState()

  // Initialize audio manager
  const audioPlayer = document.getElementById('audio-player')
  audioManager.init(audioPlayer)

  // Initialize volume manager
  volumeManager.init(initialState, audioPlayer)

  // Initialize track manager with initial state
  const trackListContainer = document.getElementById('track-list-container')
  trackManager.init(trackListContainer, initialState)

  // Initialize UI controls with initial state
  uiControls.init(initialState)

  // Start periodic state saving
  stateManager.startPeriodicSaving(
    // Function to get current state
    () => {
      // Get current track ID, ensuring it's a number or null
      const currentTrackId = trackManager.getCurrentTrackId()

      // Get current time, ensuring it's a number
      const currentTime = audioPlayer.currentTime || 0

      // Get volume state from volume manager
      const volume = volumeManager.getVolumeLevel()
      const isMuted = volumeManager.isMuted()

      // Return state in the format expected by the server
      return {
        currentTrackId,
        progressSeconds: currentTime,
        volumeLevel: volume,
        isMuted
      }
    },
    // Function to check if player is paused
    () => audioManager.isPaused()
  )
})
