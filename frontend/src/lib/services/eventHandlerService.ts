import { toast } from "svelte-sonner";
import * as apiClient from "./apiClient";
import { trackStore } from "$lib/stores/trackStore";
import { recentEventsStore } from "$lib/stores/recentEventsStore";
import { downloadStore } from "$lib/stores/downloadStore";
import type {
  MusEvent,
  DownloadReadyPayload,
  DownloadFailedPayload,
} from "$lib/types";

let openDownloadModal: (() => void) | null = null;

export function setDownloadModalOpener(opener: () => void) {
  openDownloadModal = opener;
}

/**
 * Handles incoming SSE events from the backend
 * @param payload The event payload from the SSE stream
 */
export function handleMusEvent(payload: MusEvent): void {
  // Store event in recent events store
  recentEventsStore.addEvent(payload);

  // Show toast message if provided
  if (payload.message_to_show) {
    const toastType = payload.message_level || "info";
    toast[toastType](payload.message_to_show);
  }

  // Handle actions based on action_key
  switch (payload.action_key) {
    case "track_added":
      if (payload.action_payload) {
        const track = apiClient.createTrackWithUrls(payload.action_payload);
        trackStore.addTrack(track);
      }
      break;
    case "track_updated":
      if (payload.action_payload) {
        const track = apiClient.createTrackWithUrls(payload.action_payload);
        trackStore.updateTrack(track);
      }
      break;
    case "track_deleted":
      if (
        payload.action_payload &&
        typeof payload.action_payload.id === "number"
      ) {
        trackStore.deleteTrack(payload.action_payload.id);
      }
      break;
    case "download_started":
      downloadStore.startDownload();
      break;
    case "download_completed":
      downloadStore.setCompleted();
      break;
    case "download_failed":
      if (payload.action_payload) {
        const failedPayload =
          payload.action_payload as unknown as DownloadFailedPayload;
        downloadStore.setFailed(failedPayload.error);
      }
      break;
    case "download_ready_for_review":
      if (payload.action_payload) {
        const readyPayload =
          payload.action_payload as unknown as DownloadReadyPayload;
        downloadStore.setAwaitingReview(readyPayload);
        if (openDownloadModal) {
          openDownloadModal();
        }
      }
      break;
  }
}

/**
 * Initializes the event handler service by connecting to the SSE endpoint
 * @returns The EventSource instance for the SSE connection
 */
export function initEventHandlerService(): EventSource {
  const eventSource = apiClient.connectTrackUpdateEvents(handleMusEvent);

  return eventSource;
}
