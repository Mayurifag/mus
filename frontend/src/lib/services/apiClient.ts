import type { Track, PlayerState } from "$lib/types";

export interface MusEvent {
  message_to_show: string | null;
  message_level: "success" | "error" | "info" | "warning" | null;
  action_key: string | null;
  action_payload: Record<string, unknown> | null;
}

// SSR API base URL for server-side requests (internal backend)
const SSR_API_BASE_URL =
  import.meta.env.VITE_SSR_API_BASE_URL || "http://127.0.0.1:8001/api/v1";

// Client-side API base URL (through nginx proxy)
const CLIENT_API_BASE_URL = "/api/v1";

function getApiBaseUrl(isSSR: boolean = false): string {
  return isSSR ? SSR_API_BASE_URL : CLIENT_API_BASE_URL;
}

export function getStreamUrl(trackId: number): string {
  // Stream URLs are always client-side (for audio playback)
  return `${CLIENT_API_BASE_URL}/tracks/${trackId}/stream`;
}

export async function fetchTracks(
  customFetch?: typeof fetch,
): Promise<Track[]> {
  try {
    const fetchFn = customFetch || fetch;
    // Use SSR API URL when customFetch is provided (server-side), client URL otherwise
    const apiBaseUrl = getApiBaseUrl(!!customFetch);
    const response = await fetchFn(`${apiBaseUrl}/tracks`, {
      credentials: "include",
    });
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching tracks:", error);
    return [];
  }
}

export async function fetchPlayerState(
  customFetch?: typeof fetch,
): Promise<PlayerState> {
  try {
    const fetchFn = customFetch || fetch;
    // Use SSR API URL when customFetch is provided (server-side), client URL otherwise
    const apiBaseUrl = getApiBaseUrl(!!customFetch);
    const response = await fetchFn(`${apiBaseUrl}/player/state`, {
      credentials: "include",
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
    // Beacon is always client-side
    const url = `${CLIENT_API_BASE_URL}/player/state`;
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
  // EventSource is always client-side
  const eventSource = new EventSource(
    `${CLIENT_API_BASE_URL}/events/track-updates`,
  );

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

export async function checkAuthStatus(
  customFetch?: typeof fetch,
): Promise<{ authEnabled: boolean; isAuthenticated: boolean }> {
  try {
    const fetchFn = customFetch || fetch;
    // Use SSR API URL when customFetch is provided (server-side), client URL otherwise
    const apiBaseUrl = getApiBaseUrl(!!customFetch);
    const response = await fetchFn(`${apiBaseUrl}/auth/auth-status`, {
      credentials: "include",
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
