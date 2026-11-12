import type { AudioCut, Track } from "$lib/types";
import { API_BASE_URL, PUBLIC_API_HOST } from "$lib/services/apiBase";
import {
  handleApiResponse,
  ensureApiResponse,
  createFormData,
} from "$lib/utils/apiUtils";

function createCoverUrl(url: string | null, updatedAt: number): string | null {
  if (!url) return null;

  if (url.includes("?")) {
    return `${PUBLIC_API_HOST}${url}`;
  }

  const cacheKey = Number.isFinite(updatedAt) ? updatedAt : Date.now();
  return `${PUBLIC_API_HOST}${url}?v=${cacheKey}`;
}

export function createTrackWithUrls(
  trackData: Record<string, unknown> | Track,
): Track {
  const track = trackData as Track;
  return {
    ...track,
    tags: track.tags ?? [],
    cover_small_url: createCoverUrl(track.cover_small_url, track.updated_at),
    cover_original_url: createCoverUrl(
      track.cover_original_url,
      track.updated_at,
    ),
    hls_url: `${PUBLIC_API_HOST}${track.hls_url}`,
  };
}

/** Failed refreshes reject so callers can retain the loaded library. */
export async function fetchTracks(
  category?: string | null,
  fetchFn: typeof fetch = fetch,
): Promise<Track[]> {
  const suffix = category ? `?category=${encodeURIComponent(category)}` : "";
  const response = await fetchFn(`${API_BASE_URL}/tracks${suffix}`);
  const tracks = await handleApiResponse<Track[]>(response, {
    context: "fetchTracks",
  });
  return tracks.map(createTrackWithUrls);
}

export async function updateTrack(
  trackId: string,
  updateData: {
    title?: string;
    artist?: string;
    rename_file?: boolean;
    artwork_url?: string;
    tags?: string[];
    remove_section?: AudioCut;
  },
): Promise<{ status: "queued" | "no_changes" }> {
  const response = await fetch(`${API_BASE_URL}/tracks/${trackId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(updateData),
  });
  return handleApiResponse(response, { context: "updateTrack" });
}

export async function fetchTrackCutInfo(
  trackId: string,
  signal?: AbortSignal,
): Promise<{ duration_seconds: number }> {
  const response = await fetch(`${API_BASE_URL}/tracks/${trackId}/cut-info`, {
    signal,
  });
  return handleApiResponse(response);
}

export async function fetchTrackCutPreview(
  trackId: string,
  cut: AudioCut,
  selectionOnly: boolean,
  signal?: AbortSignal,
): Promise<Blob> {
  const response = await fetch(
    `${API_BASE_URL}/tracks/${trackId}/cut-preview`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        remove_section: cut,
        selection_only: selectionOnly,
      }),
      signal,
    },
  );
  await ensureApiResponse(response, { context: "fetchTrackCutPreview" });
  return response.blob();
}

export async function deleteTrack(trackId: string): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/tracks/${trackId}`, {
    method: "DELETE",
  });
  await ensureApiResponse(response, { context: "deleteTrack" });
}

export async function fetchErroredTracks(): Promise<Track[]> {
  try {
    const response = await fetch(`${API_BASE_URL}/errors/tracks`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error("Error fetching errored tracks:", error);
    return [];
  }
}

export async function requeueTrack(trackId: string): Promise<void> {
  const response = await fetch(
    `${API_BASE_URL}/errors/tracks/${trackId}/requeue`,
    { method: "POST" },
  );
  await ensureApiResponse(response, { context: "requeueTrack" });
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

  const response = await fetch(`${API_BASE_URL}/tracks/upload`, {
    method: "POST",
    body: formData,
  });

  return handleApiResponse(response, {
    logError: true,
    context: "uploading track",
  });
}
