import type { PlayerState } from "$lib/types";
import { API_BASE_URL } from "./apiBase";
import { handleApiResponse, safeApiCall } from "$lib/utils/apiUtils";

export async function fetchPlayerState(
  fetchFn: typeof fetch = fetch,
): Promise<PlayerState> {
  const defaultState: PlayerState = {
    current_track_id: null,
    progress_seconds: 0,
    volume_level: 1,
    is_muted: false,
    is_playing: false,
    is_shuffle: false,
    is_repeat: false,
  };
  const result = await safeApiCall(
    async () => {
      const response = await fetchFn(`${API_BASE_URL}/player/state`);
      if (response.status === 404) return defaultState;
      return handleApiResponse<PlayerState>(response);
    },
    { context: "fetchPlayerState" },
  );
  return result ?? defaultState;
}

export function sendPlayerStateBeacon(state: PlayerState): void {
  if (typeof navigator !== "undefined" && navigator.sendBeacon) {
    navigator.sendBeacon(
      `${API_BASE_URL}/player/state`,
      new Blob([JSON.stringify(state)], { type: "application/json" }),
    );
  }
}
