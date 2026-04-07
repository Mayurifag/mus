import { writable } from "svelte/store";
import type { DownloadProgress } from "$lib/types";

export type DownloadState =
  | "idle"
  | "fetching_metadata"
  | "awaiting_review"
  | "downloading"
  | "completed"
  | "failed";

export interface DownloadStoreState {
  state: DownloadState;
  error: string | null;
  url: string | null;
  title: string | null;
  artist: string | null;
  thumbnailUrl: string | null;
  duration: number | null;
  progress: DownloadProgress | null;
}

const initialState: DownloadStoreState = {
  state: "idle",
  error: null,
  url: null,
  title: null,
  artist: null,
  thumbnailUrl: null,
  duration: null,
  progress: null,
};

function createDownloadStore() {
  const { subscribe, set, update } = writable<DownloadStoreState>(initialState);

  return {
    subscribe,
    setFetchingMetadata: (url: string) =>
      update((state) => ({
        ...state,
        state: "fetching_metadata",
        error: null,
        url,
      })),
    setAwaitingReview: ({
      title,
      artist,
      thumbnailUrl,
      duration,
    }: {
      title: string;
      artist: string;
      thumbnailUrl: string | null;
      duration: number | null;
    }) =>
      update((state) => ({
        ...state,
        state: "awaiting_review",
        error: null,
        title,
        artist,
        thumbnailUrl,
        duration,
      })),
    startDownload: () =>
      update((state) => ({
        ...state,
        state: "downloading",
        error: null,
        progress: null,
      })),
    setCompleted: () =>
      update((state) => ({
        ...state,
        state: "completed",
        error: null,
      })),
    setFailed: (error: string) =>
      update((state) => ({
        ...state,
        state: "failed",
        error,
      })),
    setProgress: (progress: DownloadProgress) =>
      update((s) => ({ ...s, progress })),
    reset: () => set(initialState),
  };
}

export const downloadStore = createDownloadStore();
