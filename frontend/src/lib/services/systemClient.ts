import { API_BASE_URL } from "./apiBase";
import { handleApiResponse } from "$lib/utils/apiUtils";

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

export async function fetchPermissions(
  fetchFn: typeof fetch = fetch,
): Promise<{ can_write_music_files: boolean }> {
  try {
    const response = await fetchFn(`${API_BASE_URL}/system/permissions`);
    if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
    return await response.json();
  } catch (error) {
    console.error("Error fetching permissions:", error);
    return { can_write_music_files: false };
  }
}

export async function fetchSystemInfo(
  fetchFn: typeof fetch = fetch,
): Promise<SystemInfo> {
  try {
    const response = await fetchFn(`${API_BASE_URL}/system/info`);
    if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
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
  const response = await fetch(`${API_BASE_URL}/system/yt-dlp/update`, {
    method: "POST",
  });
  return handleApiResponse(response, { context: "updateYtDlp" });
}
