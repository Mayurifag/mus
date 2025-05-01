export const volumeManager = {
  _volumeButton: null,
  _volumeSliderFill: null,
  _volumeSliderThumb: null,
  _volumeControlWrapper: null,
  _audioPlayer: null,
  _currentVolumeBeforeMute: 1.0,
  _isMuted: false,

  init (initialState, audioPlayer) {
    this._volumeButton = document.getElementById('volume-button')
    this._volumeSliderFill = document.getElementById('volume-slider-fill')
    this._volumeSliderThumb = document.querySelector('#volume-control-wrapper .custom-slider-thumb')
    this._volumeControlWrapper = document.getElementById('volume-control-wrapper')
    this._audioPlayer = audioPlayer

    // Initialize state from initial values
    this._isMuted = initialState?.isMuted || false
    this._currentVolumeBeforeMute = initialState?.volumeLevel > 0 ? initialState.volumeLevel : 1.0

    // Set initial audio player state
    this._audioPlayer.muted = this._isMuted
    this._audioPlayer.volume = this._isMuted ? 0 : this._currentVolumeBeforeMute

    // Setup event listeners
    this._setupEventListeners()

    // Update UI to match initial state
    this._updateVolumeUI()
  },

  setVolume (level) {
    // Clamp level between 0 and 1
    const clampedLevel = Math.min(1, Math.max(0, level))

    if (clampedLevel < 0.01) {
      // Effectively zero - mute the player
      this._isMuted = true
      this._audioPlayer.muted = true
      this._audioPlayer.volume = 0
    } else {
      // Non-zero volume - store and apply it
      this._currentVolumeBeforeMute = clampedLevel
      this._isMuted = false
      this._audioPlayer.muted = false
      this._audioPlayer.volume = clampedLevel
    }

    this._updateVolumeUI()
  },

  toggleMute () {
    this._isMuted = !this._isMuted
    this._audioPlayer.muted = this._isMuted

    if (this._isMuted) {
      // Store current volume if it's greater than 0
      if (this._audioPlayer.volume > 0) {
        this._currentVolumeBeforeMute = this._audioPlayer.volume
      }
      this._audioPlayer.volume = 0
    } else {
      // Restore volume, using default if stored volume was 0
      this._audioPlayer.volume = this._currentVolumeBeforeMute || 1.0
    }

    this._updateVolumeUI()
  },

  // Getters for state manager
  getVolumeLevel () {
    return this._audioPlayer.volume
  },

  isMuted () {
    return this._isMuted
  },

  _setupEventListeners () {
    // Volume button click handler
    this._volumeButton.addEventListener('click', () => this.toggleMute())

    // Slider interaction
    let isDragging = false

    const handleInteraction = (e) => {
      const rect = this._volumeControlWrapper.getBoundingClientRect()
      const pos = (e.clientX - rect.left) / rect.width
      this.setVolume(pos)
    }

    this._volumeControlWrapper.addEventListener('mousedown', (e) => {
      isDragging = true
      handleInteraction(e)
      e.preventDefault()
    })

    document.addEventListener('mousemove', (e) => {
      if (isDragging) {
        handleInteraction(e)
      }
    })

    document.addEventListener('mouseup', () => {
      isDragging = false
    })

    document.addEventListener('mouseleave', () => {
      isDragging = false
    })
  },

  _updateVolumeUI () {
    const effectiveVolume = this._isMuted ? 0 : this._audioPlayer.volume
    const percentage = effectiveVolume * 100

    // Update slider position
    this._volumeSliderFill.style.setProperty('--slider-percentage', `${percentage}%`)
    this._volumeSliderThumb.style.setProperty('--slider-percentage', `${percentage}%`)

    // Update volume icon
    if (this._isMuted || effectiveVolume === 0) {
      this._volumeButton.textContent = '🔇'
    } else if (effectiveVolume < 0.3) {
      this._volumeButton.textContent = '🔈'
    } else if (effectiveVolume < 0.7) {
      this._volumeButton.textContent = '🔉'
    } else {
      this._volumeButton.textContent = '🔊'
    }

    // Update volume controls classes
    const volumeControls = this._volumeControlWrapper.closest('.volume-controls')
    volumeControls.classList.toggle('volume-controls--muted', this._isMuted)
    volumeControls.classList.toggle('volume-controls--unmuted', !this._isMuted)
  }
}
