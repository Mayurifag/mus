import type { ArtworkSearchResult } from "$lib/types";
import { API_BASE_URL } from "./apiBase";
import { handleApiResponse } from "$lib/utils/apiUtils";

interface ArtworkQuery {
  title: string;
  artist: string;
  album?: string;
}

const resultsCache = new Map<string, ArtworkSearchResult[]>();

function searchParams({ title, artist, album }: ArtworkQuery): URLSearchParams {
  const params = new URLSearchParams({ title, artist });
  if (album) params.set("album", album);
  return params;
}

function cacheKey({ title, artist, album }: ArtworkQuery): string {
  return JSON.stringify(
    [title, artist, album ?? ""].map((value) => value.trim().toLowerCase()),
  );
}

export async function searchArtwork(
  query: ArtworkQuery,
): Promise<ArtworkSearchResult[]> {
  const response = await fetch(
    `${API_BASE_URL}/artwork/search?${searchParams(query)}`,
  );
  const results = await handleApiResponse<ArtworkSearchResult[]>(response, {
    context: "searchArtwork",
  });
  resultsCache.set(cacheKey(query), results);
  return results;
}

export async function searchArtworkStream({
  signal,
  onResults,
  ...query
}: ArtworkQuery & {
  signal?: AbortSignal;
  onResults: (results: ArtworkSearchResult[]) => void;
}): Promise<void> {
  const key = cacheKey(query);
  const cached = resultsCache.get(key);
  if (cached) onResults(cached);
  const response = await fetch(
    `${API_BASE_URL}/artwork/search/stream?${searchParams(query)}`,
    { signal },
  );
  if (!response.ok)
    await handleApiResponse(response, { context: "searchArtworkStream" });
  if (!response.body) throw new Error("Artwork stream response had no body");

  const publish = (line: string) => {
    if (!line.trim()) return;
    const results = JSON.parse(line) as ArtworkSearchResult[];
    resultsCache.set(key, results);
    onResults(results);
  };
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    while (true) {
      const { done, value } = await reader.read();
      buffer += decoder.decode(value, { stream: !done });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";
      for (const line of lines) publish(line);
      if (done) {
        publish(buffer);
        break;
      }
    }
  } finally {
    // Do not let cancellation failure replace the read/parse/callback error.
    await reader.cancel().catch(() => {});
    reader.releaseLock();
  }
}

export function preloadArtworkSearch(query: ArtworkQuery): () => void {
  if (resultsCache.has(cacheKey(query))) return () => {};
  const controller = new AbortController();
  void searchArtworkStream({
    ...query,
    signal: controller.signal,
    onResults: () => {},
  }).catch((error) => {
    if (!controller.signal.aborted)
      console.error("Error preloading artwork:", error);
  });
  return () => controller.abort();
}
