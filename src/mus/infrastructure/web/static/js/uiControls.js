import { audioManager } from './audioManager.js';
import { trackManager } from './trackManager.js';

export const uiControls = {
  init() {
    this.playPauseButton = document.getElementById('play-pause-button');
    this.prevButton = document.getElementById('prev-button');
    this.nextButton = document.getElementById('next-button');
    this.volumeButton = document.getElementById('volume-button');
    this.volumeSlider = document.getElementById('volume-slider');
    this.progressBarFill = document.getElementById('progress-bar-fill');
    this.progressBarContainer = document.getElementById('progress-bar-container');
    this.currentTimeDisplay = document.getElementById('current-time');
    this.totalDurationDisplay = document.getElementById('total-duration');

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
    const currentTime = audioManager.getCurrentTime();
    const duration = audioManager.getDuration();
    this.currentTimeDisplay.textContent = this.formatTime(currentTime);
    this.totalDurationDisplay.textContent = this.formatTime(duration);
    const progress = (currentTime / duration) * 100;
    this.progressBarFill.style.width = `${progress}%`;
  },

  formatTime(seconds) {
    if (isNaN(seconds)) return '0:00';
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
  }
};
