import type { Track, PlayerState, MusEvent } from "$lib/types";
import {
  handleApiResponse,
  createFormData,
  safeApiCall,
} from "$lib/utils/apiUtils";

const VITE_INTERNAL_API_HOST = import.meta.env.VITE_INTERNAL_API_HOST || "";
const VITE_PUBLIC_API_HOST = import.meta.env.VITE_PUBLIC_API_HOST || "";
const API_PREFIX = import.meta.env.SSR
  ? VITE_INTERNAL_API_HOST
  : VITE_PUBLIC_API_HOST;
const API_VERSION_PATH = "/api/v1";

export function getStreamUrl(trackId: number): string {
  return `${API_PREFIX}${API_VERSION_PATH}/tracks/${trackId}/stream`;
}

export function createTrackWithUrls(
  trackData: Record<string, unknown> | Track,
): Track {
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

export async function fetchTracks(
  fetchFn: typeof fetch = fetch,
): Promise<Track[]> {
  const result = await safeApiCall(
    async () => {
      const response = await fetchFn(`${API_PREFIX}${API_VERSION_PATH}/tracks`);
      const tracks: Track[] = await handleApiResponse(response);
      return tracks.map((track) => createTrackWithUrls(track));
    },
    { context: "fetchTracks" },
  );

  return result ?? [];
}

export async function fetchPlayerState(
  fetchFn: typeof fetch = fetch,
): Promise<PlayerState> {
  const defaultState: PlayerState = {
    current_track_id: null,
    progress_seconds: 0.0,
    volume_level: 1.0,
    is_muted: false,
    is_shuffle: false,
    is_repeat: false,
  };

  const result = await safeApiCall(
    async () => {
      const response = await fetchFn(
        `${API_PREFIX}${API_VERSION_PATH}/player/state`,
      );
      if (response.status === 404) {
        return defaultState;
      }
      return await handleApiResponse<PlayerState>(response);
    },
    { context: "fetchPlayerState" },
  );

  return result ?? defaultState;
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

export async function deleteTrack(trackId: number): Promise<void> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/tracks/${trackId}`,
      {
        method: "DELETE",
      },
    );

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
  } catch (error) {
    console.error("Error deleting track:", error);
    throw error;
  }
}

export async function fetchErroredTracks(): Promise<Track[]> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/errors/tracks`,
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching errored tracks:", error);
    return [];
  }
}

export async function requeueTrack(trackId: number): Promise<void> {
  try {
    const response = await fetch(
      `${API_PREFIX}${API_VERSION_PATH}/errors/tracks/${trackId}/requeue`,
      {
        method: "POST",
      },
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
  } catch (error) {
    console.error("Error requeuing track:", error);
    throw error;
  }
}

let globalEventSource: EventSource | null = null;

export function connectTrackUpdateEvents(
  onMessageCallback: (eventData: MusEvent) => void,
): EventSource {
  // Close existing connection if any
  if (globalEventSource) {
    globalEventSource.close();
  }

  const url = `${API_PREFIX}${API_VERSION_PATH}/events/track-updates`;
  const eventSource = new EventSource(url);
  globalEventSource = eventSource;

  eventSource.onmessage = (event) => {
    try {
      const eventData = JSON.parse(event.data);
      onMessageCallback(eventData);
    } catch (error) {
      console.error("Error parsing SSE event:", error);
    }
  };

  eventSource.onerror = () => {
    // Only attempt reconnection if this is still the active connection
    if (eventSource === globalEventSource) {
      setTimeout(() => {
        connectTrackUpdateEvents(onMessageCallback);
      }, 5000);
    }
  };

  return eventSource;
}

export async function fetchPermissions(fetchFn: typeof fetch = fetch): Promise<{
  can_write_music_files: boolean;
}> {
  try {
    const response = await fetchFn(
      `${API_PREFIX}${API_VERSION_PATH}/system/permissions`,
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching permissions:", error);
    return { can_write_music_files: false };
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

export async function uploadTrack(
  file: File,
  title: string,
  artist: string,
): Promise<{ success: boolean; message: string }> {
  const formData = createFormData({
    file,
    title,
    artist,
  });

  const response = await fetch(
    `${API_PREFIX}${API_VERSION_PATH}/tracks/upload`,
    {
      method: "POST",
      body: formData,
    },
  );

  return await handleApiResponse(response, {
    logError: true,
    context: "uploading track",
  });
}
