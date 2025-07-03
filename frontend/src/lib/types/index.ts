export interface Track {
  id: number;
  title: string;
  artist: string;
  duration: number;
  file_path: string;
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
  progress_seconds: number;
  volume_level: number;
  is_muted: boolean;
  is_shuffle: boolean;
  is_repeat: boolean;
}
