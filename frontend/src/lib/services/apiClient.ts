import type { Track, PlayerState } from "$lib/types";

export interface MusEvent {
  message_to_show: string | null;
  message_level: "success" | "error" | "info" | "warning" | null;
  action_key: string | null;
  action_payload: Record<string, unknown> | null;
}

function getApiBaseUrl(): string {
  const isDev = import.meta.env.DEV;

  if (isDev) {
    return "http://localhost:8000/api/v1";
  }

  return "/api/v1";
}

const API_BASE_URL = getApiBaseUrl();

export function getStreamUrl(trackId: number): string {
  return `${API_BASE_URL}/tracks/${trackId}/stream`;
}

export async function fetchTracks(): Promise<Track[]> {
  try {
    const response = await fetch(`${API_BASE_URL}/tracks`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching tracks:", error);
    return [];
  }
}

export async function fetchPlayerState(): Promise<PlayerState> {
  try {
    const response = await fetch(`${API_BASE_URL}/player/state`);
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
    // Return default player state on any error
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
    const url = `${API_BASE_URL}/player/state`;
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
  const eventSource = new EventSource(`${API_BASE_URL}/events/track-updates`);

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

    // Attempt to reconnect after a delay
    setTimeout(() => {
      console.log("Attempting to reconnect to SSE...");
      connectTrackUpdateEvents(onMessageCallback);
    }, 5000);

    // Close the errored connection
    eventSource.close();
  };

  return eventSource;
}
