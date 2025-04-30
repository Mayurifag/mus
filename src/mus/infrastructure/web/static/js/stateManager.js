class StateManager {
  constructor () {
    this._saveInterval = null
  }

  loadInitialState () {
    const body = document.body
    const trackId = body.dataset.initialTrackId
    const progressRaw = body.dataset.initialProgress
    const volumeRaw = body.dataset.initialVolume
    const isMutedRaw = body.dataset.initialMuted

    console.log('Raw initial data from body:', { trackId, progressRaw, volumeRaw, isMutedRaw })

    const progress = parseFloat(progressRaw)
    const volume = parseFloat(volumeRaw)
    const isMuted = isMutedRaw === 'true'

    const initialState = {
      currentTrackId: trackId === 'None' || trackId === '' ? null : parseInt(trackId),
      progressSeconds: isNaN(progress) ? 0 : progress,
      volumeLevel: isNaN(volume) ? 1.0 : volume,
      isMuted
    }

    console.log('Parsed initial state:', initialState)
    return initialState
  }

  // TODO: this has to be done via websocket
  async saveState (state) {
    try {
      // Ensure the state matches the expected format on the server
      const formattedState = {
        current_track_id: state.currentTrackId,
        progress_seconds: state.progressSeconds,
        volume_level: state.volumeLevel,
        is_muted: state.isMuted
      }

      console.log('Saving state:', formattedState)

      const response = await fetch('/state', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(formattedState)
      })

      if (!response.ok) {
        console.error('Failed to save state:', response.status, response.statusText)
        const errorText = await response.text()
        console.error('Error details:', errorText)
      }
    } catch (error) {
      console.error('Failed to save player state:', error)
    }
  }

  startPeriodicSaving (getCurrentState, isPaused, intervalMs = 1500) {
    this.stopPeriodicSaving()
    this._saveInterval = setInterval(() => {
      // Only save state if the player is not paused
      if (!isPaused()) {
        const state = getCurrentState()
        if (state) {
          this.saveState(state)
        }
      }
    }, intervalMs)
  }

  stopPeriodicSaving () {
    if (this._saveInterval) {
      clearInterval(this._saveInterval)
      this._saveInterval = null
    }
  }
}

export const stateManager = new StateManager()
