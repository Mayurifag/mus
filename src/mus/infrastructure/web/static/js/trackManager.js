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
      const trackItem = e.target.closest('.track-item');
      if (!trackItem) return;

      const trackId = trackItem.dataset.trackId;
      if (!trackId) return;

      const index = this.tracklist.indexOf(trackId);
      if (index === -1) return;

      if (index === this.currentIndex) {
        if (audioManager.isPaused()) {
          audioManager.play();
        } else {
          audioManager.pause();
        }
      } else {
        this.playTrackAtIndex(index);
      }
    });

    document.body.addEventListener('htmx:afterSettle', (e) => {
      if (e.target.id === 'track-list-container') {
        this.initializeTrackList();
      }
    });

    document.body.addEventListener('htmx:afterRequest', (e) => {
      if (e.target.id === 'track-list-container' && e.detail.successful) {
        setTimeout(() => this.initializeTrackList(), 100);
      }
    });
  },

  initializeTrackList() {
    const trackItems = this.trackListContainer.querySelectorAll('.track-item');
    console.log('Initializing track list with', trackItems.length, 'items');

    if (trackItems.length > 0) {
      this.tracklist = Array.from(trackItems).map(item => item.dataset.trackId);
      this.currentIndex = 0;
      const trackId = this.tracklist[0];
      const streamUrl = `/stream/${trackId}`;

      console.log('Loading initial track:', trackId);

      // Load the track but don't play it
      audioManager.loadTrack(streamUrl);

      // Update UI immediately
      this.updateTrackInfo(0);
      this.updatePlayingTrack(0);

      // Add event listener for when the track is loaded
      audioManager.audioPlayer.addEventListener('loadedmetadata', () => {
        console.log('Track metadata loaded');
        this.updateTrackInfo(0);
        this.updatePlayingTrack(0);
      }, { once: true });
    } else {
      console.log('No tracks found');
      this.currentIndex = -1;
      this.tracklist = [];
      this.updateTrackInfo(-1);
      this.updatePlayingTrack(-1);
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
    if (index < 0 || index >= this.tracklist.length) {
      document.getElementById('footer-track-title').textContent = '';
      document.getElementById('footer-track-artist').textContent = '';
      return;
    }

    const currentTrackItem = document.querySelector(`.track-item[data-track-id="${this.tracklist[index]}"]`);
    if (!currentTrackItem) return;

    const trackDetails = currentTrackItem.querySelector('.track-details');
    if (!trackDetails) return;

    const text = trackDetails.textContent.trim();
    const trackInfo = text.replace('▶', '').replace('⏸', '').trim();
    const [artist, title] = trackInfo.split(' — ').map(s => s.trim());

    document.getElementById('footer-track-title').textContent = title || '';
    document.getElementById('footer-track-artist').textContent = artist || '';
  },

  updatePlayingTrack(index) {
    // Reset all track items
    document.querySelectorAll('.track-item').forEach(item => {
      item.classList.remove('playing');
      const playButton = item.querySelector('.play-button');
      if (playButton) playButton.textContent = '▶';
    });

    if (index < 0 || index >= this.tracklist.length) return;

    const currentTrackItem = document.querySelector(`.track-item[data-track-id="${this.tracklist[index]}"]`);
    if (!currentTrackItem) return;

    currentTrackItem.classList.add('playing');
    const playButton = currentTrackItem.querySelector('.play-button');
    if (playButton) {
      playButton.textContent = audioManager.isPaused() ? '▶' : '⏸';
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
