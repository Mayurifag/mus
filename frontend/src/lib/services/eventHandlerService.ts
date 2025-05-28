import { toast } from "svelte-sonner";
import * as apiClient from "./apiClient";
import { trackStore } from "$lib/stores/trackStore";

type MessageLevel = "success" | "error" | "info" | "warning";

interface MusEvent {
  message_to_show: string | null;
  message_level: MessageLevel | null;
  action_key: string | null;
  action_payload: Record<string, unknown> | null;
}

/**
 * Handles incoming SSE events from the backend
 * @param payload The event payload from the SSE stream
 */
export function handleMusEvent(payload: MusEvent): void {
  // Show toast message if provided
  if (payload.message_to_show) {
    const toastType = payload.message_level || "info";
    toast[toastType](payload.message_to_show);
  }

  // Handle actions based on action_key
  if (payload.action_key === "reload_tracks") {
    // Only reload tracks if we actually need to (avoid redundant calls)
    apiClient
      .fetchTracks()
      .then((tracks) => {
        trackStore.setTracks(tracks);
      })
      .catch((error) => {
        console.error("Failed to reload tracks:", error);
      });
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
