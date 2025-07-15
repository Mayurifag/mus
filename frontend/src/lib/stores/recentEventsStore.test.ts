import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { recentEventsStore } from "./recentEventsStore";
import type { MusEvent } from "$lib/types";

// Mock crypto.randomUUID
Object.defineProperty(global, "crypto", {
  value: {
    randomUUID: vi.fn(() => "test-uuid"),
  },
});

describe("recentEventsStore", () => {
  beforeEach(() => {
    recentEventsStore.clear();
    vi.clearAllMocks();
  });

  it("should start with empty events", () => {
    const events = get(recentEventsStore);
    expect(events).toEqual([]);
  });

  it("should add events with id and timestamp", () => {
    const testEvent: MusEvent = {
      message_to_show: "Test message",
      message_level: "success",
      action_key: "track_added",
      action_payload: null,
    };

    recentEventsStore.addEvent(testEvent);

    const events = get(recentEventsStore);
    expect(events).toHaveLength(1);
    expect(events[0]).toMatchObject({
      ...testEvent,
      id: "test-uuid",
    });
    expect(events[0].timestamp).toBeTypeOf("number");
  });

  it("should add new events at the beginning", () => {
    const event1: MusEvent = {
      message_to_show: "First event",
      message_level: "info",
      action_key: null,
      action_payload: null,
    };

    const event2: MusEvent = {
      message_to_show: "Second event",
      message_level: "success",
      action_key: null,
      action_payload: null,
    };

    recentEventsStore.addEvent(event1);
    recentEventsStore.addEvent(event2);

    const events = get(recentEventsStore);
    expect(events).toHaveLength(2);
    expect(events[0].message_to_show).toBe("Second event");
    expect(events[1].message_to_show).toBe("First event");
  });

  it("should limit events to maximum of 10", () => {
    // Add 12 events
    for (let i = 0; i < 12; i++) {
      const event: MusEvent = {
        message_to_show: `Event ${i}`,
        message_level: "info",
        action_key: null,
        action_payload: null,
      };
      recentEventsStore.addEvent(event);
    }

    const events = get(recentEventsStore);
    expect(events).toHaveLength(10);
    expect(events[0].message_to_show).toBe("Event 11");
    expect(events[9].message_to_show).toBe("Event 2");
  });

  it("should clear all events", () => {
    const testEvent: MusEvent = {
      message_to_show: "Test message",
      message_level: "info",
      action_key: null,
      action_payload: null,
    };

    recentEventsStore.addEvent(testEvent);
    expect(get(recentEventsStore)).toHaveLength(1);

    recentEventsStore.clear();
    expect(get(recentEventsStore)).toEqual([]);
  });
});
