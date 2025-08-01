import { writable } from "svelte/store";

export type DownloadState =
  | "idle"
  | "downloading"
  | "completed"
  | "awaiting_review"
  | "failed";

export interface DownloadStoreState {
  state: DownloadState;
  error: string | null;
  tempId: string | null;
  suggestedTitle: string | null;
  suggestedArtist: string | null;
  coverDataUrl: string | null;
}

const initialState: DownloadStoreState = {
  state: "idle",
  error: null,
  tempId: null,
  suggestedTitle: null,
  suggestedArtist: null,
  coverDataUrl: null,
};

function createDownloadStore() {
  const { subscribe, set, update } = writable<DownloadStoreState>(initialState);

  return {
    subscribe,
    startDownload: () =>
      update((state) => ({
        ...state,
        state: "downloading",
        error: null,
        tempId: null,
        suggestedTitle: null,
        suggestedArtist: null,
        coverDataUrl: null,
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
    setAwaitingReview: (payload: {
      tempId: string;
      title: string;
      artist: string;
      coverDataUrl: string;
    }) =>
      update((state) => ({
        ...state,
        state: "awaiting_review",
        error: null,
        tempId: payload.tempId,
        suggestedTitle: payload.title,
        suggestedArtist: payload.artist,
        coverDataUrl: payload.coverDataUrl,
      })),
    reset: () => set(initialState),
  };
}

export const downloadStore = createDownloadStore();
