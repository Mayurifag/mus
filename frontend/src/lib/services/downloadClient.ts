import { API_BASE_URL } from "./apiBase";
import { handleApiResponse } from "$lib/utils/apiUtils";

export interface TrackMetadata {
  title: string;
  artist: string;
  thumbnail_url: string | null;
  duration: number | null;
  tags?: { name: string; display_name: string }[];
}

export async function startDownload(url: string): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/downloads/url`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url }),
  });
  await handleApiResponse(response, { context: "startDownload" });
}

export async function fetchMetadata(url: string): Promise<TrackMetadata> {
  const response = await fetch(`${API_BASE_URL}/downloads/metadata`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url }),
  });
  return handleApiResponse(response, { context: "fetchMetadata" });
}

export async function confirmDownload(
  url: string,
  title: string,
  artist: string,
  artworkUrl?: string,
  tags: string[] = [],
): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/downloads/confirm`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url, title, artist, artwork_url: artworkUrl, tags }),
  });
  await handleApiResponse(response, { context: "confirmDownload" });
}
