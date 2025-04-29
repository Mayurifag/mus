document.addEventListener('DOMContentLoaded', () => {
  const audioPlayer = document.getElementById('audio-player');
  const trackListContainer = document.getElementById('track-list-container');

  trackListContainer.addEventListener('click', (e) => {
    const playButton = e.target.closest('.play-button');
    if (!playButton) return;

    const filePath = playButton.dataset.filePath;
    if (!filePath) return;

    const streamUrl = `/stream/${encodeURIComponent(filePath)}`;
    audioPlayer.src = streamUrl;
    audioPlayer.load();
    audioPlayer.play();
  });
});
