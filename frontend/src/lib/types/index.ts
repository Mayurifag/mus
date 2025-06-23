export interface Track {
  id: number;
  title: string;
  artist: string;
  duration: number; // Duration in seconds
  has_cover: boolean;
  cover_small_url: string | null;
  cover_original_url: string | null;
}

export interface TimeRange {
  start: number;
  end: number;
}

export interface PlayerState {
  current_track_id: number | null;
  progress_seconds: number; // Current playback position in seconds
  volume_level: number; // Volume level between 0.0 and 1.0
  is_muted: boolean;
  is_shuffle: boolean;
  is_repeat: boolean;
}
