import type { Track, PlayerState } from "$lib/types";
import { dev } from "$app/environment";
import { browser } from "$app/environment";

export interface MusEvent {
  message_to_show: string | null;
  message_level: "success" | "error" | "info" | "warning" | null;
  action_key: string | null;
  action_payload: Record<string, unknown> | null;
}

function getAuthHeaders(): Record<string, string> {
  if (!browser) return {};

  const token = localStorage.getItem("auth_token");
  if (token) {
    return { Authorization: `Bearer ${token}` };
  }
  return {};
}

const VITE_INTERNAL_API_HOST =
  import.meta.env.VITE_INTERNAL_API_HOST || "http://localhost:8000";
const VITE_PUBLIC_API_HOST =
  import.meta.env.VITE_PUBLIC_API_HOST || "http://localhost:8000";

const API_VERSION = "/api/v1";

function getApiBaseUrl(): string {
  if (import.meta.env.SSR) {
    return `${VITE_INTERNAL_API_HOST}${API_VERSION}`;
  }

  if (dev) {
    return `${VITE_PUBLIC_API_HOST}${API_VERSION}`;
  }

  return API_VERSION;
}

const API_BASE_PREFIX = dev
  ? `${VITE_PUBLIC_API_HOST}${API_VERSION}`
  : API_VERSION;

export function getStreamUrl(trackId: number): string {
  return `${API_BASE_PREFIX}/tracks/${trackId}/stream`;
}

export async function fetchTracks(): Promise<Track[]> {
  try {
    const apiBaseUrl = getApiBaseUrl();
    const response = await fetch(`${apiBaseUrl}/tracks`, {
      headers: {
        ...getAuthHeaders(),
      },
    });
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const tracks: Track[] = await response.json();

    for (const track of tracks) {
      if (track.cover_small_url) {
        track.cover_small_url = `${API_BASE_PREFIX}${track.cover_small_url}`;
      }
      if (track.cover_original_url) {
        track.cover_original_url = `${API_BASE_PREFIX}${track.cover_original_url}`;
      }
    }

    return tracks;
  } catch (error) {
    console.error("Error fetching tracks:", error);
    return [];
  }
}

export async function fetchPlayerState(): Promise<PlayerState> {
  try {
    const apiBaseUrl = getApiBaseUrl();
    const response = await fetch(`${apiBaseUrl}/player/state`, {
      headers: {
        ...getAuthHeaders(),
      },
    });
    if (response.status === 404) {
      // Return default player state if none exists
      return {
        current_track_id: null,
        progress_seconds: 0.0,
        volume_level: 1.0,
        is_muted: false,
        is_shuffle: false,
        is_repeat: false,
      };
    }
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching player state:", error);
    return {
      current_track_id: null,
      progress_seconds: 0.0,
      volume_level: 1.0,
      is_muted: false,
      is_shuffle: false,
      is_repeat: false,
    };
  }
}

export function sendPlayerStateBeacon(state: PlayerState): void {
  if (typeof navigator !== "undefined" && navigator.sendBeacon) {
    const url = `${API_BASE_PREFIX}/player/state`;
    const blob = new Blob([JSON.stringify(state)], {
      type: "application/json",
    });
    navigator.sendBeacon(url, blob);
  }
}

/**
 * Connects to the SSE endpoint for track updates
 * @param onMessageCallback Callback function to handle incoming SSE events
 * @returns The EventSource instance
 */
export function connectTrackUpdateEvents(
  onMessageCallback: (eventData: MusEvent) => void,
): EventSource {
  const url = `${API_BASE_PREFIX}/events/track-updates`;
  const eventSource = new EventSource(url);

  eventSource.onmessage = (event) => {
    try {
      const eventData = JSON.parse(event.data);
      onMessageCallback(eventData);
    } catch (error) {
      console.error("Error parsing SSE event:", error);
    }
  };

  eventSource.onerror = (error) => {
    console.error("SSE connection error:", error);

    setTimeout(() => {
      console.log("Attempting to reconnect to SSE...");
      connectTrackUpdateEvents(onMessageCallback);
    }, 5000);

    eventSource.close();
  };

  return eventSource;
}

export async function checkAuthStatus(): Promise<{
  authEnabled: boolean;
  isAuthenticated: boolean;
}> {
  try {
    const apiBaseUrl = getApiBaseUrl();
    const response = await fetch(`${apiBaseUrl}/auth/auth-status`, {
      headers: {
        ...getAuthHeaders(),
      },
    });
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data = await response.json();
    return {
      authEnabled: data.auth_enabled,
      isAuthenticated: data.authenticated,
    };
  } catch (error) {
    console.error("Error checking auth status:", error);
    return { authEnabled: false, isAuthenticated: false };
  }
}
