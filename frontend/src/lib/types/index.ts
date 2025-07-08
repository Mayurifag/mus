export interface Track {
  id: number;
  title: string;
  artist: string;
  duration: number;
  file_path: string;
  updated_at: number;
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

export interface TrackHistory {
  id: number;
  track_id: number;
  title: string;
  artist: string;
  duration: number;
  changed_at: number;
  event_type: string;
  filename: string;
  changes?: Record<string, unknown> | null;
  full_snapshot?: Record<string, unknown> | null;
}

export interface MusEvent {
  message_to_show: string | null;
  message_level: "success" | "error" | "info" | "warning" | null;
  action_key: string | null;
  action_payload: Record<string, unknown> | null;
}
