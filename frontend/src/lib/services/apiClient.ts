import type {
  ArtworkSearchResult,
  Track,
  PlayerState,
  MusEvent,
} from "$lib/types";
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
const artworkResultsCache = new Map<string, ArtworkSearchResult[]>();

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
    artwork_url?: string;
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

export async function searchArtwork({
  title,
  artist,
  album,
}: {
  title: string;
  artist: string;
  album?: string;
}): Promise<ArtworkSearchResult[]> {
  const params = new URLSearchParams({ title, artist });
  if (album) {
    params.set("album", album);
  }

  const cacheKey = artworkSearchCacheKey({ title, artist, album });
  const response = await fetch(
    `${API_PREFIX}${API_VERSION_PATH}/artwork/search?${params.toString()}`,
  );
  const results = await handleApiResponse<ArtworkSearchResult[]>(response, {
    context: "searchArtwork",
  });
  artworkResultsCache.set(cacheKey, results);
  return results;
}

export async function searchArtworkStream({
  title,
  artist,
  album,
  signal,
  onResults,
}: {
  title: string;
  artist: string;
  album?: string;
  signal?: AbortSignal;
  onResults: (results: ArtworkSearchResult[]) => void;
}): Promise<void> {
  const params = new URLSearchParams({ title, artist });
  if (album) {
    params.set("album", album);
  }
  const cacheKey = artworkSearchCacheKey({ title, artist, album });
  const cachedResults = artworkResultsCache.get(cacheKey);
  if (cachedResults) {
    onResults(cachedResults);
  }

  const response = await fetch(
    `${API_PREFIX}${API_VERSION_PATH}/artwork/search/stream?${params.toString()}`,
    { signal },
  );
  if (!response.ok) {
    await handleApiResponse(response, { context: "searchArtworkStream" });
  }
  if (!response.body) {
    throw new Error("Artwork stream response had no body");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    buffer += decoder.decode(value, { stream: !done });
    const lines = buffer.split("\n");
    buffer = lines.pop() ?? "";

    for (const line of lines) {
      if (line.trim()) {
        const results = JSON.parse(line) as ArtworkSearchResult[];
        artworkResultsCache.set(cacheKey, results);
        onResults(results);
      }
    }

    if (done) {
      if (buffer.trim()) {
        const results = JSON.parse(buffer) as ArtworkSearchResult[];
        artworkResultsCache.set(cacheKey, results);
        onResults(results);
      }
      break;
    }
  }
}

export function preloadArtworkSearch({
  title,
  artist,
  album,
}: {
  title: string;
  artist: string;
  album?: string;
}): () => void {
  const cacheKey = artworkSearchCacheKey({ title, artist, album });
  if (artworkResultsCache.has(cacheKey)) {
    return () => {};
  }

  const controller = new AbortController();
  void searchArtworkStream({
    title,
    artist,
    album,
    signal: controller.signal,
    onResults: () => {},
  }).catch((error) => {
    if (!controller.signal.aborted) {
      console.error("Error preloading artwork:", error);
    }
  });

  return () => controller.abort();
}

function artworkSearchCacheKey({
  title,
  artist,
  album,
}: {
  title: string;
  artist: string;
  album?: string;
}): string {
  return [title, artist, album ?? ""]
    .map((value) => value.trim().toLowerCase())
    .join("|");
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
let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;

export function closeTrackUpdateEvents(
  eventSource: EventSource | null = globalEventSource,
): void {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
    reconnectTimeout = null;
  }

  if (eventSource) {
    eventSource.close();
  }

  if (!eventSource || eventSource === globalEventSource) {
    globalEventSource = null;
  }
}

export function connectTrackUpdateEvents(
  onMessageCallback: (eventData: MusEvent) => void,
  onOpenCallback?: () => void,
): EventSource {
  if (globalEventSource) {
    closeTrackUpdateEvents(globalEventSource);
  }

  const url = `${API_PREFIX}${API_VERSION_PATH}/events/track-updates`;
  const eventSource = new EventSource(url, { withCredentials: true });
  globalEventSource = eventSource;

  eventSource.onmessage = (event) => {
    try {
      const eventData = JSON.parse(event.data);
      onMessageCallback(eventData);
    } catch (error) {
      console.error("Error parsing SSE event:", error);
    }
  };

  eventSource.onopen = () => {
    onOpenCallback?.();
  };

  eventSource.onerror = () => {
    if (eventSource === globalEventSource && reconnectTimeout === null) {
      reconnectTimeout = setTimeout(() => {
        reconnectTimeout = null;
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

export async function startDownload(url: string): Promise<void> {
  const result = await safeApiCall(
    async () => {
      const response = await fetch(
        `${API_PREFIX}${API_VERSION_PATH}/downloads/url`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ url }),
        },
      );
      await handleApiResponse(response);
    },
    { context: "startDownload" },
  );

  if (result === null) {
    throw new Error("Failed to start download");
  }
}

export interface TrackMetadata {
  title: string;
  artist: string;
  thumbnail_url: string | null;
  duration: number | null;
}

export async function fetchMetadata(url: string): Promise<TrackMetadata> {
  const response = await fetch(
    `${API_PREFIX}${API_VERSION_PATH}/downloads/metadata`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ url }),
    },
  );
  return await handleApiResponse<TrackMetadata>(response, {
    context: "fetchMetadata",
  });
}

export async function confirmDownload(
  url: string,
  title: string,
  artist: string,
  artworkUrl?: string,
): Promise<void> {
  const result = await safeApiCall(
    async () => {
      const response = await fetch(
        `${API_PREFIX}${API_VERSION_PATH}/downloads/confirm`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ url, title, artist, artwork_url: artworkUrl }),
        },
      );
      await handleApiResponse(response);
    },
    { context: "confirmDownload" },
  );

  if (result === null) {
    throw new Error("Failed to confirm download");
  }
}

export interface SystemInfo {
  app_date: string;
  commit_sha: string | null;
  music_dir: {
    path: string;
    exists: boolean;
    is_directory: boolean;
    is_empty: boolean | null;
    can_write: boolean;
    warning: string | null;
  };
  yt_dlp_version: string | null;
}

export async function fetchSystemInfo(
  fetchFn: typeof fetch = fetch,
): Promise<SystemInfo> {
  try {
    const response = await fetchFn(
      `${API_PREFIX}${API_VERSION_PATH}/system/info`,
    );
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching system info:", error);
    return {
      app_date: "unknown",
      commit_sha: null,
      music_dir: {
        path: "",
        exists: false,
        is_directory: false,
        is_empty: null,
        can_write: false,
        warning: "System info unavailable",
      },
      yt_dlp_version: null,
    };
  }
}

export async function updateYtDlp(): Promise<{
  yt_dlp_version: string | null;
  output: string;
}> {
  const response = await fetch(
    `${API_PREFIX}${API_VERSION_PATH}/system/yt-dlp/update`,
    { method: "POST" },
  );
  return await handleApiResponse(response, { context: "updateYtDlp" });
}

export async function uploadTrack(
  file: File,
  title: string,
  artist: string,
  artworkUrl?: string,
): Promise<{ success: boolean; message: string }> {
  const formData = createFormData({
    file,
    title,
    artist,
  });
  if (artworkUrl) {
    formData.append("artwork_url", artworkUrl);
  }

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
