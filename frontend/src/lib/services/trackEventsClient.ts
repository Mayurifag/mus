import { writable } from "svelte/store";
import type { MusEvent } from "$lib/types";
import { API_BASE_URL } from "./apiBase";

export type SseConnectionStatus =
  "disconnected" | "connecting" | "connected" | "reconnecting";
export const trackUpdatesConnectionStatus =
  writable<SseConnectionStatus>("disconnected");

let globalEventSource: EventSource | null = null;
let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;

/** Closes the singleton subscription, including a source replaced by reconnection. */
export function closeTrackUpdateEvents(
  eventSource: EventSource | null = globalEventSource,
): void {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
    reconnectTimeout = null;
  }
  eventSource?.close();
  if (globalEventSource !== eventSource) globalEventSource?.close();
  globalEventSource = null;
  trackUpdatesConnectionStatus.set("disconnected");
}

export function connectTrackUpdateEvents(
  onMessageCallback: (eventData: MusEvent) => void,
  onOpenCallback?: () => void,
): EventSource {
  if (globalEventSource) closeTrackUpdateEvents(globalEventSource);
  trackUpdatesConnectionStatus.set("connecting");
  const eventSource = new EventSource(`${API_BASE_URL}/events/track-updates`);
  globalEventSource = eventSource;
  eventSource.onmessage = (event) => {
    if (eventSource !== globalEventSource) return;
    try {
      const eventData = JSON.parse(event.data);
      console.info(
        "SSE track update event",
        eventData.action_key,
        eventData.message_to_show ?? eventData.action_payload?.filename ?? "",
      );
      onMessageCallback(eventData);
    } catch (error) {
      console.error("Error parsing SSE event:", error);
    }
  };
  eventSource.onopen = () => {
    if (eventSource !== globalEventSource) return;
    if (reconnectTimeout !== null) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }
    console.info("SSE track updates connected");
    trackUpdatesConnectionStatus.set("connected");
    onOpenCallback?.();
  };
  eventSource.onerror = (event) => {
    if (eventSource !== globalEventSource) return;
    console.warn("SSE track updates error", event);
    trackUpdatesConnectionStatus.set("reconnecting");
    if (reconnectTimeout === null) {
      reconnectTimeout = setTimeout(() => {
        reconnectTimeout = null;
        connectTrackUpdateEvents(onMessageCallback, onOpenCallback);
      }, 5000);
    }
  };
  return eventSource;
}
