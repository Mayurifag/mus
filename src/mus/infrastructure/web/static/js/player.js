document.addEventListener('DOMContentLoaded', () => {
  const audioPlayer = document.getElementById('audio-player');
  const trackListContainer = document.getElementById('track-list-container');
  const playPauseButton = document.getElementById('play-pause-button');
  const prevButton = document.getElementById('prev-button');
  const nextButton = document.getElementById('next-button');
  const volumeButton = document.getElementById('volume-button');
  const volumeSlider = document.getElementById('volume-slider');
  const progressBarFill = document.getElementById('progress-bar-fill');
  const progressBarContainer = document.getElementById('progress-bar-container');
  const nowPlayingInfo = document.getElementById('now-playing-info');
  const currentTimeDisplay = document.getElementById('current-time');

  let tracklist = [];
  let currentIndex = -1;
  let currentVolume = 1.0;
  let isMuted = false;
  audioPlayer.volume = currentVolume;

  function formatTime(seconds) {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
  }

  // Update time display
  audioPlayer.addEventListener('timeupdate', () => {
    currentTimeDisplay.textContent = formatTime(audioPlayer.currentTime);
    const progress = (audioPlayer.currentTime / audioPlayer.duration) * 100;
    progressBarFill.style.width = `${progress}%`;
  });

  // Play/Pause button functionality
  playPauseButton.addEventListener('click', () => {
    if (audioPlayer.paused) {
      if (currentIndex === -1 && tracklist.length > 0) {
        playTrackAtIndex(0);
      } else {
        audioPlayer.play();
        updatePlayingTrack(currentIndex);
      }
    } else {
      audioPlayer.pause();
      updatePlayingTrack(currentIndex);
    }
  });

  // Update play/pause button icon
  audioPlayer.addEventListener('play', () => {
    playPauseButton.textContent = '⏸';
  });

  audioPlayer.addEventListener('pause', () => {
    playPauseButton.textContent = '▶';
  });

  // Volume controls
  volumeSlider.addEventListener('input', (e) => {
    const volume = parseFloat(e.target.value);
    audioPlayer.volume = volume;
    currentVolume = volume;
    if (volume === 0) {
      volumeButton.textContent = '🔇';
      isMuted = true;
    } else {
      volumeButton.textContent = '🔊';
      isMuted = false;
    }
  });

  volumeButton.addEventListener('click', () => {
    isMuted = !isMuted;
    audioPlayer.muted = isMuted;
    volumeButton.textContent = isMuted ? '🔇' : '🔊';
  });

  // Next/Previous controls
  prevButton.addEventListener('click', () => {
    if (currentIndex > 0) {
      playTrackAtIndex(currentIndex - 1);
    }
  });

  nextButton.addEventListener('click', () => {
    if (currentIndex < tracklist.length - 1) {
      playTrackAtIndex(currentIndex + 1);
    }
  });

  // Auto-play next track
  audioPlayer.addEventListener('ended', () => {
    if (currentIndex < tracklist.length - 1) {
      playTrackAtIndex(currentIndex + 1);
    }
  });

  // Update progress bar
  audioPlayer.addEventListener('timeupdate', () => {
    const progress = (audioPlayer.currentTime / audioPlayer.duration) * 100;
    progressBarFill.style.width = `${progress}%`;
  });

  // Update progress bar when metadata is loaded
  audioPlayer.addEventListener('loadedmetadata', () => {
    const progress = (audioPlayer.currentTime / audioPlayer.duration) * 100;
    progressBarFill.style.width = `${progress}%`;
  });

  // Click on progress bar to seek
  progressBarContainer.addEventListener('click', (e) => {
    const rect = progressBarContainer.getBoundingClientRect();
    const pos = (e.clientX - rect.left) / rect.width;
    audioPlayer.currentTime = pos * audioPlayer.duration;
  });

  // Track list click handler
  trackListContainer.addEventListener('click', (e) => {
    const playButton = e.target.closest('.play-button');
    if (!playButton) return;

    const trackId = playButton.dataset.trackId;
    if (!trackId) return;

    const index = tracklist.indexOf(trackId);
    if (index === -1) return;

    playTrackAtIndex(index);
  });

  // HTMX event listener for track list initialization
  document.body.addEventListener('htmx:afterSettle', (e) => {
    if (e.target.id === 'track-list-container') {
      const trackItems = trackListContainer.querySelectorAll('.track-item');
      if (trackItems.length > 0) {
        tracklist = Array.from(trackItems).map(item => {
          const button = item.querySelector('.play-button');
          return button.dataset.trackId;
        });
        currentIndex = 0;
        const trackId = tracklist[0];
        const streamUrl = `/stream/${trackId}`;
        audioPlayer.src = streamUrl;
        audioPlayer.load();
        updateTrackInfo(0);
        playPauseButton.textContent = '▶';
        document.querySelectorAll('.play-button').forEach(button => {
          button.textContent = '▶';
        });
      } else {
        currentIndex = -1;
        tracklist = [];
        nowPlayingInfo.textContent = '';
        playPauseButton.disabled = true;
        prevButton.disabled = true;
        nextButton.disabled = true;
      }
    }
  });

  function updateTrackInfo(index) {
    const currentTrackItem = document.querySelector(`.play-button[data-track-id="${tracklist[index]}"]`)?.closest('.track-item');

    if (currentTrackItem) {
      const trackDetails = currentTrackItem.querySelector('.track-details');
      if (trackDetails) {
        const text = trackDetails.textContent.trim();
        const trackInfo = text.replace('▶', '').trim();
        const [artist, title] = trackInfo.split(' — ').map(s => s.trim());
        nowPlayingInfo.textContent = `${artist} - ${title}`;
      } else {
        nowPlayingInfo.textContent = '';
      }
    } else {
      nowPlayingInfo.textContent = '';
    }
  }

  function updatePlayingTrack(index) {
    document.querySelectorAll('.track-item.playing').forEach(item => {
      item.classList.remove('playing');
      const playButton = item.querySelector('.play-button');
      if (playButton) playButton.textContent = '▶';
    });

    const currentTrackItem = document.querySelector(`.play-button[data-track-id="${tracklist[index]}"]`)?.closest('.track-item');
    if (currentTrackItem) {
      currentTrackItem.classList.add('playing');
      const playButton = currentTrackItem.querySelector('.play-button');
      if (playButton) playButton.textContent = '⏸';
    }
  }

  function playTrackAtIndex(index) {
    if (index < 0 || index >= tracklist.length) return;

    currentIndex = index;
    const trackId = tracklist[index];
    const streamUrl = `/stream/${trackId}`;
    audioPlayer.src = streamUrl;
    audioPlayer.load();
    audioPlayer.play().catch(error => {
      console.error('Error playing audio:', error);
    });

    updateTrackInfo(index);
    updatePlayingTrack(index);
  }

  // Update track info when metadata is loaded
  audioPlayer.addEventListener('loadedmetadata', () => {
    updateTrackInfo(currentIndex);
    const progress = (audioPlayer.currentTime / audioPlayer.duration) * 100;
    progressBarFill.style.width = `${progress}%`;
  });

  // Update track info when track ends
  audioPlayer.addEventListener('ended', () => {
    if (currentIndex < tracklist.length - 1) {
      playTrackAtIndex(currentIndex + 1);
    } else {
      updateTrackInfo(-1);
    }
  });
});
