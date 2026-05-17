import type { Track } from "$lib/types";
import { formatArtistsForDisplay } from "$lib/utils/formatters";

export interface MediaSessionHandlers {
  play: () => void;
  pause: () => void;
  nextTrack: () => void;
  previousTrack: () => void;
  seekTo: (time: number) => void;
}

function hasMediaSession(): boolean {
  return typeof navigator !== "undefined" && "mediaSession" in navigator;
}

export function setupMediaSessionHandlers(
  handlers: MediaSessionHandlers,
): void {
  if (!hasMediaSession()) return;

  navigator.mediaSession.setActionHandler("play", handlers.play);
  navigator.mediaSession.setActionHandler("pause", handlers.pause);
  navigator.mediaSession.setActionHandler("nexttrack", handlers.nextTrack);
  navigator.mediaSession.setActionHandler(
    "previoustrack",
    handlers.previousTrack,
  );
  navigator.mediaSession.setActionHandler("seekto", (details) => {
    if (details.seekTime !== undefined) {
      handlers.seekTo(details.seekTime);
    }
  });
  navigator.mediaSession.setActionHandler("seekforward", null);
  navigator.mediaSession.setActionHandler("seekbackward", null);
}

export function updateMediaSessionMetadata(track: Track): void {
  if (!hasMediaSession()) return;

  const artwork: MediaImage[] = [];
  if (track.cover_original_url) {
    artwork.push({
      src: track.cover_original_url,
      sizes: "512x512",
      type: "image/webp",
    });
  }

  const metadata: MediaMetadataInit = {
    title: track.title,
    artist: formatArtistsForDisplay(track.artist),
  };

  if (artwork.length > 0) {
    metadata.artwork = artwork;
  }

  navigator.mediaSession.metadata = new MediaMetadata(metadata);
}

export function updateMediaSessionPlaybackState(isPlaying: boolean): void {
  if (!hasMediaSession()) return;

  navigator.mediaSession.playbackState = isPlaying ? "playing" : "paused";
}

export function clearMediaSession(): void {
  if (!hasMediaSession()) return;

  navigator.mediaSession.setActionHandler("play", null);
  navigator.mediaSession.setActionHandler("pause", null);
  navigator.mediaSession.setActionHandler("nexttrack", null);
  navigator.mediaSession.setActionHandler("previoustrack", null);
  navigator.mediaSession.setActionHandler("seekto", null);
  navigator.mediaSession.metadata = null;
  navigator.mediaSession.playbackState = "none";
}
