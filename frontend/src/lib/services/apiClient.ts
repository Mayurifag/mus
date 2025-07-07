import type { Track, PlayerState, TrackHistory, MusEvent } from "$lib/types";

const VITE_INTERNAL_API_HOST = import.meta.env.VITE_INTERNAL_API_HOST || "";
const VITE_PUBLIC_API_HOST = import.meta.env.VITE_PUBLIC_API_HOST || "";
const API_PREFIX = import.meta.env.SSR
  ? VITE_INTERNAL_API_HOST
  : VITE_PUBLIC_API_HOST;
const API_VERSION_PATH = "/api/v1";

export function getStreamUrl(trackId: number): string {
  return `${API_PREFIX}${API_VERSION_PATH}/tracks/${trackId}/stream`;
}

export function createTrackWithUrls(trackData: Record<string, unknown> | Track): Track {
  const track = trackData as Track;
  return {
    ...track,
    cover_small_url: track.cover_small_url
      ? `${VITE_PUBLIC_API_HOST}${track.cover_small_url}`
      : null,
    cover_original_url: track.cover_original_url
      ? `${VITE_PUBLIC_API_HOST}${track.cover_original_url}`
      : null,
  };
}

export async function fetchTracks(): Promise<Track[]> {
  try {
    const response = await fetch(`${API_PREFIX}${API_VERSION_PATH}/tracks`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const tracks: Track[] = await response.json();

    return tracks.map(track => createTrackWithUrls(track));
  } catch (error) {
    console.error("Error fetching tracks:", error);
    return [];
  }
}

export async function fetchPlayerState(): Promise<PlayerState> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/player/state`,
    );
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
    const url = `${API_PREFIX}${API_VERSION_PATH}/player/state`;
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
export async function updateTrack(
  trackId: number,
  updateData: {
    title?: string;
    artist?: string;
    rename_file?: boolean;
  },
): Promise<{ status: string; track?: Track }> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/tracks/${trackId}`,
      {
        method: "PATCH",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(updateData),
      },
    );

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return await response.json();
  } catch (error) {
    console.error("Error updating track:", error);
    throw error;
  }
}

export async function fetchTrackHistory(
  trackId: number,
): Promise<TrackHistory[]> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/tracks/${trackId}/history`,
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching track history:", error);
    return [];
  }
}

export function connectTrackUpdateEvents(
  onMessageCallback: (eventData: MusEvent) => void,
): EventSource {
  const url = `${API_PREFIX}${API_VERSION_PATH}/events/track-updates`;
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

export async function fetchPermissions(): Promise<{
  can_write_files: boolean;
}> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/system/permissions`,
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching permissions:", error);
    return { can_write_files: false };
  }
}

export async function fetchMagicLinkUrl(): Promise<string> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/auth/qr-code-url`,
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data = await response.json();
    return data.url;
  } catch (error) {
    console.error("Error fetching magic link URL:", error);
    return "";
  }
}
