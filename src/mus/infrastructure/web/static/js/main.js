import { audioManager } from './audioManager.js';
import { trackManager } from './trackManager.js';
import { uiControls } from './uiControls.js';

document.addEventListener('DOMContentLoaded', () => {
  // Initialize audio manager
  const audioPlayer = document.getElementById('audio-player');
  audioManager.init(audioPlayer);

  // Initialize track manager
  const trackListContainer = document.getElementById('track-list-container');
  trackManager.init(trackListContainer);

  // Initialize UI controls
  uiControls.init();
});
