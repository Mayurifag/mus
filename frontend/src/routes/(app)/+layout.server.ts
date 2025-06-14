import { error } from "@sveltejs/kit";
import type { LayoutServerLoad } from "./$types";
import {
  fetchTracks,
  fetchPlayerState,
  checkAuthStatus,
} from "$lib/services/apiClient";

export const load: LayoutServerLoad = async () => {
  try {
    // First check auth status
    const authStatus = await checkAuthStatus();

    // Only fetch protected data if authenticated
    if (authStatus.authEnabled && !authStatus.isAuthenticated) {
      // User needs to authenticate - return minimal data
      return {
        tracks: [],
        playerState: {
          current_track_id: null,
          progress_seconds: 0.0,
          volume_level: 1.0,
          is_muted: false,
          is_shuffle: false,
          is_repeat: false,
        },
        authStatus,
      };
    }
    // User is authenticated or auth is disabled - fetch all data
    const [tracks, playerState] = await Promise.all([
      fetchTracks(),
      fetchPlayerState(),
    ]);

    return {
      tracks,
      playerState,
      authStatus,
    };
  } catch (err: unknown) {
    console.error("Error in load function (Promise.all or unexpected):", err);
    throw error(500, { message: "Failed to load initial data" });
  }
};
