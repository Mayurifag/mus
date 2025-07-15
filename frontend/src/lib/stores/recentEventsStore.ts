import { writable } from "svelte/store";
import type { MusEvent } from "$lib/types";

export interface RecentEvent extends MusEvent {
  id: string;
  timestamp: number;
}

const MAX_EVENTS = 10;

function createRecentEventsStore() {
  const { subscribe, update } = writable<RecentEvent[]>([]);

  return {
    subscribe,
    addEvent: (event: MusEvent) => {
      const recentEvent: RecentEvent = {
        ...event,
        id: crypto.randomUUID(),
        timestamp: Date.now(),
      };

      update((events) => {
        const newEvents = [recentEvent, ...events];
        return newEvents.slice(0, MAX_EVENTS);
      });
    },
    clear: () => update(() => []),
  };
}

export const recentEventsStore = createRecentEventsStore();
