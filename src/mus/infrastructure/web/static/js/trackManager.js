import { audioManager } from './audioManager.js';

export const trackManager = {
  tracklist: [],
  currentIndex: -1,

  init(trackListContainer) {
    this.trackListContainer = trackListContainer;
    this.setupEventListeners();
  },

  setupEventListeners() {
    this.trackListContainer.addEventListener('click', (e) => {
      const playButton = e.target.closest('.play-button');
      if (!playButton) return;

      const trackId = playButton.dataset.trackId;
      if (!trackId) return;

      const index = this.tracklist.indexOf(trackId);
      if (index === -1) return;

      this.playTrackAtIndex(index);
    });

    document.body.addEventListener('htmx:afterSettle', (e) => {
      if (e.target.id === 'track-list-container') {
        this.initializeTrackList();
      }
    });
  },

  initializeTrackList() {
    const trackItems = this.trackListContainer.querySelectorAll('.track-item');
    if (trackItems.length > 0) {
      this.tracklist = Array.from(trackItems).map(item => {
        const button = item.querySelector('.play-button');
        return button.dataset.trackId;
      });
      this.currentIndex = 0;
      this.playTrackAtIndex(0);
    } else {
      this.currentIndex = -1;
      this.tracklist = [];
    }
  },

  playTrackAtIndex(index) {
    if (index < 0 || index >= this.tracklist.length) return;

    this.currentIndex = index;
    const trackId = this.tracklist[index];
    const streamUrl = `/stream/${trackId}`;
    audioManager.loadTrack(streamUrl);
    audioManager.play().catch(error => {
      console.error('Error playing audio:', error);
    });

    this.updateTrackInfo(index);
    this.updatePlayingTrack(index);
  },

  updateTrackInfo(index) {
    const currentTrackItem = document.querySelector(`.play-button[data-track-id="${this.tracklist[index]}"]`)?.closest('.track-item');
    const trackTitle = document.getElementById('footer-track-title');
    const trackArtist = document.getElementById('footer-track-artist');

    if (currentTrackItem) {
      const trackDetails = currentTrackItem.querySelector('.track-details');
      if (trackDetails) {
        const text = trackDetails.textContent.trim();
        const trackInfo = text.replace('▶', '').trim();
        const [artist, title] = trackInfo.split(' — ').map(s => s.trim());
        trackTitle.textContent = title;
        trackArtist.textContent = artist;
      } else {
        trackTitle.textContent = '';
        trackArtist.textContent = '';
      }
    } else {
      trackTitle.textContent = '';
      trackArtist.textContent = '';
    }
  },

  updatePlayingTrack(index) {
    document.querySelectorAll('.track-item.playing').forEach(item => {
      item.classList.remove('playing');
      const playButton = item.querySelector('.play-button');
      if (playButton) playButton.textContent = '▶';
    });

    const currentTrackItem = document.querySelector(`.play-button[data-track-id="${this.tracklist[index]}"]`)?.closest('.track-item');
    if (currentTrackItem) {
      currentTrackItem.classList.add('playing');
      const playButton = currentTrackItem.querySelector('.play-button');
      if (playButton) playButton.textContent = '⏸';
    }
  },

  getNextTrack() {
    if (this.currentIndex < this.tracklist.length - 1) {
      return this.currentIndex + 1;
    }
    return -1;
  },

  getPreviousTrack() {
    if (this.currentIndex > 0) {
      return this.currentIndex - 1;
    }
    return -1;
  }
};
