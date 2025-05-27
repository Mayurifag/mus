import type { Track, PlayerState } from "$lib/types";
import xior from "xior";

// Define the structure of SSE events
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

// Create a global xior instance with the base URL
const api = xior.create({
  baseURL: API_BASE_URL,
  timeout: 10000,
});

// Create a separate instance with shorter timeout for fire-and-forget saves
const quickApi = xior.create({
  baseURL: API_BASE_URL, // Shorter timeout
  timeout: 3000,
});

export function getStreamUrl(trackId: number): string {
  return `${API_BASE_URL}/tracks/${trackId}/stream`;
}

export async function fetchTracks(): Promise<Track[]> {
  try {
    const response = await api.get("/tracks");
    return response.data;
  } catch (error) {
    console.error("Error fetching tracks:", error);
    return [];
  }
}

export async function fetchPlayerState(): Promise<PlayerState | null> {
  try {
    const response = await api.get("/player/state");
    return response.data;
  } catch (error) {
    if (
      error &&
      typeof error === "object" &&
      "response" in error &&
      error.response &&
      typeof error.response === "object" &&
      "status" in error.response &&
      error.response.status === 404
    ) {
      return null;
    }
    console.error("Error fetching player state:", error);
    return null;
  }
}

// Fire-and-forget version for non-blocking saves
export function savePlayerStateAsync(state: PlayerState): void {
  quickApi.post("/player/state", state).catch((error) => {
    // Silent failure - just log to console without affecting UI
    if (error.name !== "XiorTimeoutError") {
      console.warn("Player state save failed (non-critical):", error.message);
    }
    // Timeout errors are completely ignored to prevent console spam
  });
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
