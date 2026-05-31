import { trackStore } from "$lib/stores/trackStore";
import { sendPlayerStateBeacon } from "$lib/services/apiClient";
import type { AudioService } from "$lib/services/AudioService";
import type { PlayerState, Track } from "$lib/types";

export const defaultPlayerState: PlayerState = {
  current_track_id: null,
  progress_seconds: 0.0,
  volume_level: 1.0,
  is_muted: false,
  is_shuffle: false,
  is_repeat: false,
};

export function restorePlayerState(
  audioService: AudioService | undefined,
  tracks: Track[],
  playerState: PlayerState,
): string | null {
  if (!audioService) return null;

  const {
    current_track_id,
    progress_seconds,
    volume_level,
    is_muted,
    is_shuffle,
    is_repeat,
  } = playerState;

  audioService.initializeState(volume_level, is_muted, is_repeat);
  trackStore.setShuffle(is_shuffle);

  if (current_track_id !== null) {
    const trackIndex = tracks.findIndex(
      (track: Track) => track.id === current_track_id,
    );
    if (trackIndex >= 0) {
      trackStore.setCurrentTrackIndex(trackIndex);
      audioService.updateAudioSource(
        tracks[trackIndex],
        false,
        progress_seconds,
      );
      return current_track_id;
    }
  } else if (tracks.length > 0) {
    trackStore.setCurrentTrackIndex(0);
    audioService.updateAudioSource(tracks[0], false);
    return tracks[0].id;
  }

  return null;
}

export function savePlayerState(
  audioService: AudioService | undefined,
  currentTrack: Track | null,
  isShuffle: boolean,
): void {
  if (!currentTrack || !audioService) return;

  sendPlayerStateBeacon({
    current_track_id: currentTrack.id,
    progress_seconds: audioService.currentTime,
    volume_level: audioService.volume,
    is_muted: audioService.isMuted,
    is_shuffle: isShuffle,
    is_repeat: audioService.isRepeat,
  });
}
