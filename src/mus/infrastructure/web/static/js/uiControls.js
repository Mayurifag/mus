import { audioManager } from './audioManager.js';
import { trackManager } from './trackManager.js';

export const uiControls = {
  init(initialState = null) {
    this.playPauseButton = document.getElementById('play-pause-button');
    this.prevButton = document.getElementById('prev-button');
    this.nextButton = document.getElementById('next-button');
    this.volumeButton = document.getElementById('volume-button');
    this.volumeSlider = document.getElementById('volume-slider');
    this.progressBarFill = document.getElementById('progress-bar-fill');
    this.progressBarContainer = document.getElementById('progress-bar-container');
    this.currentTimeDisplay = document.getElementById('current-time');
    this.totalDurationDisplay = document.getElementById('total-duration');

    if (initialState) {
      console.log("Initializing UI Controls with state:", initialState);

      // Set volume
      this.volumeSlider.value = initialState.volumeLevel;
      audioManager.setVolume(initialState.volumeLevel);

      // Directly set mute state based on initial state
      if (initialState.isMuted) {
        audioManager.isMuted = true;
        audioManager.audioPlayer.muted = true;
        this.volumeButton.textContent = '🔇';
      } else {
        audioManager.isMuted = false;
        audioManager.audioPlayer.muted = false;
        this.volumeButton.textContent = '🔊';
      }

      // Ensure volume slider reflects mute state if volume was 0
      if (initialState.volumeLevel === 0) {
        this.volumeButton.textContent = '🔇';
      }

      // Set initial progress if available
      if (initialState.progressSeconds > 0) {
        // We'll update the progress bar once the audio metadata is loaded
        // This is handled in the trackManager's metadataListener
        console.log("Initial progress seconds:", initialState.progressSeconds);
      }
    }

    this.setupEventListeners();
  },

  setupEventListeners() {
    // Play/Pause button
    this.playPauseButton.addEventListener('click', () => {
      if (audioManager.isPaused()) {
        if (trackManager.currentIndex === -1 && trackManager.tracklist.length > 0) {
          trackManager.playTrackAtIndex(0);
        } else {
          audioManager.play();
          trackManager.updatePlayingTrack(trackManager.currentIndex);
        }
      } else {
        audioManager.pause();
        trackManager.updatePlayingTrack(trackManager.currentIndex);
      }
    });

    // Volume controls
    this.volumeSlider.addEventListener('input', (e) => {
      const volume = parseFloat(e.target.value);
      const isMuted = audioManager.setVolume(volume);
      this.volumeButton.textContent = isMuted ? '🔇' : '🔊';
    });

    this.volumeButton.addEventListener('click', () => {
      const isMuted = audioManager.toggleMute();
      this.volumeButton.textContent = isMuted ? '🔇' : '🔊';
    });

    // Navigation controls
    this.prevButton.addEventListener('click', () => {
      const prevIndex = trackManager.getPreviousTrack();
      if (prevIndex !== -1) {
        trackManager.playTrackAtIndex(prevIndex);
      }
    });

    this.nextButton.addEventListener('click', () => {
      const nextIndex = trackManager.getNextTrack();
      if (nextIndex !== -1) {
        trackManager.playTrackAtIndex(nextIndex);
      }
    });

    // Progress bar
    this.progressBarContainer.addEventListener('click', (e) => {
      const rect = this.progressBarContainer.getBoundingClientRect();
      const pos = (e.clientX - rect.left) / rect.width;
      audioManager.seek(pos);
    });

    // Audio player events
    audioManager.audioPlayer.addEventListener('play', () => {
      this.playPauseButton.textContent = '⏸';
      trackManager.updatePlayingTrack(trackManager.currentIndex);
    });

    audioManager.audioPlayer.addEventListener('pause', () => {
      this.playPauseButton.textContent = '▶';
      trackManager.updatePlayingTrack(trackManager.currentIndex);
    });

    audioManager.audioPlayer.addEventListener('timeupdate', () => {
      this.updateTimeDisplay();
    });

    audioManager.audioPlayer.addEventListener('loadedmetadata', () => {
      this.updateTimeDisplay();
    });

    audioManager.audioPlayer.addEventListener('ended', () => {
      const nextIndex = trackManager.getNextTrack();
      if (nextIndex !== -1) {
        trackManager.playTrackAtIndex(nextIndex);
      } else {
        trackManager.updatePlayingTrack(trackManager.currentIndex);
      }
    });
  },

  updateTimeDisplay() {
    // Added checks for NaN/Infinity which can happen before duration is known
    const currentTime = audioManager.getCurrentTime();
    const duration = audioManager.getDuration();
    if (isNaN(currentTime) || !isFinite(currentTime)) return;

    this.currentTimeDisplay.textContent = this.formatTime(currentTime);

    if (isNaN(duration) || !isFinite(duration) || duration === 0) {
      this.totalDurationDisplay.textContent = '0:00';
      this.progressBarFill.style.width = '0%';
    } else {
      this.totalDurationDisplay.textContent = this.formatTime(duration);
      const progress = (currentTime / duration) * 100;
      this.progressBarFill.style.width = `${Math.min(100, Math.max(0, progress))}%`; // Clamp progress
    }
  },

  formatTime(seconds) {
    if (isNaN(seconds) || !isFinite(seconds)) return '0:00'; // Handle invalid input
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
  }
};
