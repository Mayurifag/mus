import { audioManager } from './audioManager.js'
import { trackManager } from './trackManager.js'

export const uiControls = {
  init (initialState = null) {
    this.playPauseButton = document.getElementById('play-pause-button')
    this.prevButton = document.getElementById('prev-button')
    this.nextButton = document.getElementById('next-button')
    this.volumeButton = document.getElementById('volume-button')
    this.volumeSliderFill = document.getElementById('volume-slider-fill')
    this.volumeSliderThumb = document.querySelector('#volume-control-wrapper .custom-slider-thumb')
    this.volumeControlWrapper = document.getElementById('volume-control-wrapper')
    this.progressBarFill = document.getElementById('progress-bar-fill')
    this.progressBarThumb = document.querySelector('#progress-bar-container .custom-slider-thumb')
    this.progressBarContainer = document.getElementById('progress-bar-container')
    this.progressWrapper = document.querySelector('.progress-section .slider-interaction-wrapper')
    this.currentTimeDisplay = document.getElementById('current-time')
    this.totalDurationDisplay = document.getElementById('total-duration')

    if (initialState) {
      console.log('Initializing UI Controls with state:', initialState)

      // Set volume
      this.updateVolumeUI(initialState.volumeLevel)
      audioManager.setVolume(initialState.volumeLevel)

      // Directly set mute state based on initial state
      if (initialState.isMuted) {
        audioManager.isMuted = true
        audioManager.audioPlayer.muted = true
        const volumeControls = this.volumeControlWrapper.closest('.volume-controls')
        volumeControls.classList.add('volume-controls--muted')
        volumeControls.classList.remove('volume-controls--unmuted')
      } else {
        audioManager.isMuted = false
        audioManager.audioPlayer.muted = false
        const volumeControls = this.volumeControlWrapper.closest('.volume-controls')
        volumeControls.classList.remove('volume-controls--muted')
        volumeControls.classList.add('volume-controls--unmuted')
      }

      // Ensure volume slider reflects mute state if volume was 0
      if (initialState.volumeLevel === 0) {
        const volumeControls = this.volumeControlWrapper.closest('.volume-controls')
        volumeControls.classList.add('volume-controls--muted')
        volumeControls.classList.remove('volume-controls--unmuted')
      }

      // Set initial progress if available
      if (initialState.progressSeconds > 0) {
        console.log('Initial progress seconds:', initialState.progressSeconds)
      }
    }

    this.setupEventListeners()
  },

  setupEventListeners () {
    // Play/Pause button
    this.playPauseButton.addEventListener('click', () => {
      if (audioManager.isPaused()) {
        if (trackManager.currentIndex === -1 && trackManager.tracklist.length > 0) {
          trackManager.playTrackAtIndex(0)
        } else {
          audioManager.play()
          trackManager.updatePlayingTrack(trackManager.currentIndex)
        }
      } else {
        audioManager.pause()
        trackManager.updatePlayingTrack(trackManager.currentIndex)
      }
    })

    // Volume controls
    this.setupSliderInteraction(
      this.volumeControlWrapper,
      this.volumeSliderFill,
      this.volumeSliderThumb,
      (value) => {
        const isMuted = audioManager.setVolume(value)
        const volumeControls = this.volumeControlWrapper.closest('.volume-controls')
        volumeControls.classList.toggle('volume-controls--muted', isMuted)
        volumeControls.classList.toggle('volume-controls--unmuted', !isMuted)
      }
    )

    this.volumeButton.addEventListener('click', () => {
      const isMuted = audioManager.toggleMute()
      const volumeControls = this.volumeControlWrapper.closest('.volume-controls')
      volumeControls.classList.toggle('volume-controls--muted', isMuted)
      volumeControls.classList.toggle('volume-controls--unmuted', !isMuted)
    })

    // Navigation controls
    this.prevButton.addEventListener('click', () => {
      const prevIndex = trackManager.getPreviousTrack()
      if (prevIndex !== -1) {
        trackManager.playTrackAtIndex(prevIndex)
      }
    })

    this.nextButton.addEventListener('click', () => {
      const nextIndex = trackManager.getNextTrack()
      if (nextIndex !== -1) {
        trackManager.playTrackAtIndex(nextIndex)
      }
    })

    // Progress bar
    this.setupSliderInteraction(
      this.progressWrapper,
      this.progressBarFill,
      this.progressBarThumb,
      (value) => {
        audioManager.seek(value)
      }
    )

    // Audio player events
    audioManager.audioPlayer.addEventListener('play', () => {
      this.playPauseButton.classList.add('is-playing')
      this.playPauseButton.classList.remove('is-paused')
      trackManager.updatePlayingTrack(trackManager.currentIndex)
    })

    audioManager.audioPlayer.addEventListener('pause', () => {
      this.playPauseButton.classList.remove('is-playing')
      this.playPauseButton.classList.add('is-paused')
      trackManager.updatePlayingTrack(trackManager.currentIndex)
    })

    audioManager.audioPlayer.addEventListener('timeupdate', () => {
      this.updateTimeDisplay()
    })

    audioManager.audioPlayer.addEventListener('loadedmetadata', () => {
      this.updateTimeDisplay()
    })

    audioManager.audioPlayer.addEventListener('ended', () => {
      const nextIndex = trackManager.getNextTrack()
      if (nextIndex !== -1) {
        trackManager.playTrackAtIndex(nextIndex)
      } else {
        trackManager.updatePlayingTrack(trackManager.currentIndex)
      }
    })
  },

  setupSliderInteraction (wrapperElement, fillElement, thumbElement, updateCallback) {
    let isDragging = false

    const updateSliderUI = (value) => {
      const clampedValue = Math.min(1, Math.max(0, value))
      const percentage = clampedValue * 100
      fillElement.style.setProperty('--slider-percentage', `${percentage}%`)
      thumbElement.style.setProperty('--slider-percentage', `${percentage}%`)
    }

    const handleInteraction = (e) => {
      const rect = wrapperElement.getBoundingClientRect()
      const pos = (e.clientX - rect.left) / rect.width
      updateSliderUI(pos)
      updateCallback(pos)
    }

    wrapperElement.addEventListener('mousedown', (e) => {
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

  updateVolumeUI (volume) {
    this.updateSliderUI(this.volumeSliderFill, this.volumeSliderThumb, volume)
    const volumeControls = this.volumeControlWrapper.closest('.volume-controls')
    volumeControls.classList.toggle('volume-controls--muted', volume === 0)
    volumeControls.classList.toggle('volume-controls--unmuted', volume > 0)
  },

  updateSliderUI (fillElement, thumbElement, value) {
    const clampedValue = Math.min(1, Math.max(0, value))
    const percentage = clampedValue * 100
    fillElement.style.setProperty('--slider-percentage', `${percentage}%`)
    thumbElement.style.setProperty('--slider-percentage', `${percentage}%`)
  },

  updateTimeDisplay () {
    // Added checks for NaN/Infinity which can happen before duration is known
    const currentTime = audioManager.getCurrentTime()
    const duration = audioManager.getDuration()
    if (isNaN(currentTime) || !isFinite(currentTime)) return

    this.currentTimeDisplay.textContent = this.formatTime(currentTime)

    if (isNaN(duration) || !isFinite(duration) || duration === 0) {
      this.totalDurationDisplay.textContent = '0:00'
      this.updateSliderUI(this.progressBarFill, this.progressBarThumb, 0)
    } else {
      this.totalDurationDisplay.textContent = this.formatTime(duration)
      const progress = currentTime / duration
      this.updateSliderUI(this.progressBarFill, this.progressBarThumb, progress)
    }
  },

  formatTime (seconds) {
    if (isNaN(seconds) || !isFinite(seconds)) return '0:00' // Handle invalid input
    const minutes = Math.floor(seconds / 60)
    const remainingSeconds = Math.floor(seconds % 60)
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`
  }
}
