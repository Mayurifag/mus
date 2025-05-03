import { audioManager } from './audioManager.js'
import { uiControls } from './uiControls.js'
import { mediaSessionManager } from './mediaSessionManager.js'

export const trackManager = {
  tracklist: [],
  currentIndex: -1,
  initialState: null,

  init (trackListContainer, initialState = null) {
    this.trackListContainer = trackListContainer
    this.initialState = initialState
    this.setupEventListeners()
  },

  setupEventListeners () {
    this.trackListContainer.addEventListener('click', (e) => {
      const trackItem = e.target.closest('.track-item')
      if (!trackItem) return

      const trackId = trackItem.dataset.trackId
      if (!trackId) return

      const index = this.tracklist.findIndex(id => id === trackId)
      if (index === -1) return

      if (index === this.currentIndex) {
        if (audioManager.isPaused()) {
          audioManager.play()
        } else {
          audioManager.pause()
        }
        this.updatePlayingTrack(this.currentIndex)
      } else {
        this.playTrackAtIndex(index)
      }
    })

    document.body.addEventListener('htmx:afterSettle', (e) => {
      if (e.target.id === 'track-list-container' || this.trackListContainer.contains(e.target)) {
        console.log('htmx:afterSettle triggered for track list')
        if (this.trackListContainer.querySelector('.track-item')) {
          this.initializeTrackList()
        } else {
          console.log('Track list container updated but no tracks found.')
          this.tracklist = []
          this.currentIndex = -1
          this.updateTrackInfo(-1)
          this.updatePlayingTrack(-1)
        }
      }
    })
  },

  initializeTrackList () {
    if (this.initializing) return
    this.initializing = true

    const trackItems = this.trackListContainer.querySelectorAll('.track-item')
    console.log('Initializing track list with', trackItems.length, 'items')

    if (trackItems.length > 0) {
      this.tracklist = Array.from(trackItems).map(item => item.dataset.trackId)

      let initialIndex = -1
      if (this.initialState?.currentTrackId !== null && this.initialState?.currentTrackId !== undefined) {
        const initialTrackIdStr = String(this.initialState.currentTrackId)
        initialIndex = this.tracklist.findIndex(id => id === initialTrackIdStr)
        console.log(`Attempting to find initial track ID ${initialTrackIdStr}, found index: ${initialIndex}`)
      } else {
        console.log('No valid initial track ID provided in state.')
      }

      if (initialIndex !== -1) {
        this.currentIndex = initialIndex
      } else if (this.tracklist.length > 0) {
        this.currentIndex = 0
        console.log('Initial track ID not found or null, defaulting to index 0')
      } else {
        this.currentIndex = -1
      }

      if (this.currentIndex === -1) {
        console.log('No valid initial track index found.')
        this.updateTrackInfo(-1)
        this.updatePlayingTrack(-1)
        this.initializing = false
        return
      }

      const trackId = this.tracklist[this.currentIndex]
      const streamUrl = `/stream/${trackId}`

      console.log(`Loading initial track: ID=${trackId}, Index=${this.currentIndex}, URL=${streamUrl}`)

      audioManager.loadTrack(streamUrl)
      this.updateTrackInfo(this.currentIndex)
      this.updatePlayingTrack(this.currentIndex)

      const metadataListener = () => {
        console.log('Track metadata loaded. Duration:', audioManager.getDuration())
        uiControls.updateTimeDisplay()

        if (this.initialState?.progressSeconds) {
          const progress = parseFloat(this.initialState.progressSeconds)
          const duration = audioManager.getDuration()
          if (!isNaN(progress) && progress > 0 && !isNaN(duration) && duration > 0 && progress < duration) {
            setTimeout(() => {
              audioManager.audioPlayer.currentTime = progress
              console.log('Seeked to initial progress:', progress)
              uiControls.updateTimeDisplay()
            }, 100)
          } else {
            console.log(`Skipping seek: Invalid/zero progress (${progress}) or duration (${duration}).`)
          }
        } else {
          console.log('No initial progress to seek to.')
        }
        this.updateTrackInfo(this.currentIndex)
        this.updatePlayingTrack(this.currentIndex)
      }

      const errorListener = (e) => {
        console.error('Error loading initial audio track:', e)
      }

      audioManager.audioPlayer.removeEventListener('loadedmetadata', metadataListener)
      audioManager.audioPlayer.removeEventListener('error', errorListener)

      audioManager.audioPlayer.addEventListener('loadedmetadata', metadataListener, { once: true })
      audioManager.audioPlayer.addEventListener('error', errorListener, { once: true })
    } else {
      console.log('No tracks found in the list container')
      this.currentIndex = -1
      this.tracklist = []
      this.updateTrackInfo(-1)
      this.updatePlayingTrack(-1)
    }

    this.initializing = false
  },

  playTrackAtIndex (index) {
    if (index < 0 || index >= this.tracklist.length) {
      console.warn(`Attempted to play invalid index: ${index}`)
      return
    }

    this.currentIndex = index
    const trackId = this.tracklist[index]
    const streamUrl = `/stream/${trackId}`
    console.log(`Playing track: ID=${trackId}, Index=${index}, URL=${streamUrl}`)
    audioManager.loadTrack(streamUrl)
    const playPromise = audioManager.play()

    if (playPromise !== undefined) {
      playPromise.then(_ => {
        console.log('Playback started for track', trackId)
        this.updateTrackInfo(index)
        this.updatePlayingTrack(index)
        this.preloadAdjacentTracks(index)
      }).catch(error => {
        console.error('Error playing audio:', error)
        this.updatePlayingTrack(index)
      })
    } else {
      this.updateTrackInfo(index)
      this.updatePlayingTrack(index)
      this.preloadAdjacentTracks(index)
    }
  },

  preloadAdjacentTracks (currentIndex) {
    const currentTrackId = this.tracklist[currentIndex]
    const nextTrackId = this.tracklist[currentIndex + 1]
    const prevTrackId = this.tracklist[currentIndex - 1]

    if (nextTrackId) {
      audioManager.preloadTrack(nextTrackId)
    }
    if (prevTrackId) {
      audioManager.preloadTrack(prevTrackId)
    }

    audioManager.clearOldPreloadedTracks(currentTrackId, nextTrackId, prevTrackId)
  },

  updateTrackInfo (index) {
    const titleElement = document.getElementById('footer-track-title')
    const artistElement = document.getElementById('footer-track-artist')
    const coverElement = document.getElementById('footer-cover-img')

    if (!titleElement || !artistElement || !coverElement) {
      console.error('Footer track info elements not found')
      return
    }

    if (index < 0 || index >= this.tracklist.length) {
      titleElement.textContent = 'No Track Selected'
      artistElement.textContent = ''
      coverElement.src = '/static/images/placeholder.svg'
      mediaSessionManager.updateMetadata('No Track Selected', '', '/static/android-chrome-512x512.png')
      return
    }

    const trackId = this.tracklist[index]
    const currentTrackItem = document.querySelector(`.track-item[data-track-id="${trackId}"]`)
    if (!currentTrackItem) {
      console.warn(`Track item with ID ${trackId} not found in DOM for info update.`)
      titleElement.textContent = 'Loading...'
      artistElement.textContent = ''
      coverElement.src = '/static/images/placeholder.svg'
      mediaSessionManager.updateMetadata('Loading...', '', '/static/android-chrome-512x512.png')
      return
    }

    const title = currentTrackItem.dataset.title
    const artist = currentTrackItem.dataset.artist
    const hasCover = currentTrackItem.dataset.hasCover === 'true'

    if (!title || !artist) {
      console.warn(`Title or artist data not found for track ID ${trackId}.`)
      titleElement.textContent = 'Error'
      artistElement.textContent = ''
      coverElement.src = '/static/images/placeholder.svg'
      mediaSessionManager.updateMetadata('Error', '', '/static/android-chrome-512x512.png')
      return
    }

    titleElement.textContent = title
    artistElement.textContent = artist
    const artworkUrl = hasCover ? `/covers/medium/${trackId}.webp` : '/static/android-chrome-512x512.png'
    coverElement.src = artworkUrl
    mediaSessionManager.updateMetadata(title, artist, artworkUrl)
  },

  updatePlayingTrack (index) {
    // TODO: fix this, we should optionally give here index of prev track and only update that item, not all of them
    const trackItems = this.trackListContainer.querySelectorAll('.track-item')
    trackItems.forEach(item => {
      item.classList.remove('is-active')
      const button = item.querySelector('.play-button')
      if (button) {
        button.setAttribute('data-playing', 'false')
      }
    })

    if (index >= 0 && index < this.tracklist.length) {
      const trackId = this.tracklist[index]
      const activeItem = document.querySelector(`.track-item[data-track-id="${trackId}"]`)
      if (activeItem) {
        activeItem.classList.add('is-active')
        const button = activeItem.querySelector('.play-button')
        if (button) {
          button.setAttribute('data-playing', !audioManager.isPaused())
        }
      }
    }
  },

  getNextTrack () {
    if (this.tracklist.length === 0) return -1
    if (this.currentIndex < this.tracklist.length - 1) {
      return this.currentIndex + 1
    }
    return -1
  },

  getPreviousTrack () {
    if (this.tracklist.length === 0) return -1
    if (this.currentIndex > 0) {
      return this.currentIndex - 1
    }
    return -1
  },

  getCurrentTrackId () {
    if (this.currentIndex >= 0 && this.currentIndex < this.tracklist.length) {
      const id = parseInt(this.tracklist[this.currentIndex])
      return isNaN(id) ? null : id
    }
    return null
  }
}
