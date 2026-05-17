export interface Track {
  id: number;
  title: string;
  artist: string;
  duration: number;
  filename: string;
  updated_at: number;
  has_cover: boolean;
  cover_small_url: string | null;
  cover_original_url: string | null;
}

export interface ArtworkSearchResult {
  id: string;
  source: string;
  title: string;
  artist: string | null;
  image_url: string;
  thumbnail_url: string;
  width: number | null;
  height: number | null;
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

export interface MusEvent {
  message_to_show: string | null;
  message_level: "success" | "error" | "info" | "warning" | null;
  action_key: string | null;
  action_payload: Record<string, unknown> | null;
}

export interface DownloadFailedPayload {
  error: string;
}

export interface DownloadProgress {
  percent: number;
  speed: string;
  eta: string;
}

export interface DownloadProgressPayload {
  percent: number;
  speed: string;
  eta: string;
}
