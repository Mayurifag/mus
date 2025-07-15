import { error } from "@sveltejs/kit";
import type { LayoutServerLoad } from "./$types";
import {
  fetchTracks,
  fetchPlayerState,
  fetchPermissions,
} from "$lib/services/apiClient";

export const load: LayoutServerLoad = async () => {
  try {
    const [tracks, playerState, permissions] = await Promise.all([
      fetchTracks(),
      fetchPlayerState(),
      fetchPermissions(),
    ]);

    return {
      tracks,
      playerState,
      permissions,
    };
  } catch (err: unknown) {
    console.error("Error in load function (Promise.all or unexpected):", err);
    throw error(500, { message: "Failed to load initial data" });
  }
};
