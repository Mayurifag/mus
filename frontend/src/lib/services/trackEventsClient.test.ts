import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import {
  closeTrackUpdateEvents,
  connectTrackUpdateEvents,
  trackUpdatesConnectionStatus,
} from "./trackEventsClient";

let sources: EventSource[];

beforeEach(() => {
  vi.useFakeTimers();
  sources = [];
  vi.stubGlobal(
    "EventSource",
    vi.fn(function () {
      const source = {
        close: vi.fn(),
        onopen: null,
        onmessage: null,
        onerror: null,
      } as unknown as EventSource;
      sources.push(source);
      return source;
    }),
  );
  vi.spyOn(console, "info").mockImplementation(() => {});
  vi.spyOn(console, "warn").mockImplementation(() => {});
});

afterEach(() => {
  closeTrackUpdateEvents();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("track event connection", () => {
  it("reports connection state and forwards parsed events", () => {
    const onMessage = vi.fn();
    const onOpen = vi.fn();
    const source = connectTrackUpdateEvents(onMessage, onOpen);
    expect(get(trackUpdatesConnectionStatus)).toBe("connecting");
    source.onopen?.call(source, new Event("open"));
    expect(get(trackUpdatesConnectionStatus)).toBe("connected");
    expect(onOpen).toHaveBeenCalledOnce();
    source.onmessage?.call(
      source,
      new MessageEvent("message", {
        data: JSON.stringify({ action_key: "track_added" }),
      }),
    );
    expect(onMessage).toHaveBeenCalledWith({ action_key: "track_added" });
  });

  it("reconnects once after an error and closes the old source", () => {
    const source = connectTrackUpdateEvents(vi.fn());
    source.onerror?.call(source, new Event("error"));
    source.onerror?.call(source, new Event("error"));
    expect(get(trackUpdatesConnectionStatus)).toBe("reconnecting");
    vi.advanceTimersByTime(5000);
    expect(sources).toHaveLength(2);
    expect(source.close).toHaveBeenCalledOnce();
  });

  it("cancels reconnection when the connection is closed", () => {
    const source = connectTrackUpdateEvents(vi.fn());
    source.onerror?.call(source, new Event("error"));
    closeTrackUpdateEvents(source);
    vi.advanceTimersByTime(5000);
    expect(sources).toHaveLength(1);
    expect(get(trackUpdatesConnectionStatus)).toBe("disconnected");
  });

  it("closes the active connection when cleanup receives the original source after reconnecting", () => {
    const source = connectTrackUpdateEvents(vi.fn());
    source.onerror?.call(source, new Event("error"));
    vi.advanceTimersByTime(5000);
    closeTrackUpdateEvents(source);
    expect(sources[1].close).toHaveBeenCalledOnce();
    sources[1].onerror?.call(sources[1], new Event("error"));
    vi.advanceTimersByTime(5000);
    expect(sources).toHaveLength(2);
  });

  it("cancels the fallback reconnect when EventSource reconnects itself", () => {
    const source = connectTrackUpdateEvents(vi.fn());
    source.onerror?.call(source, new Event("error"));
    source.onopen?.call(source, new Event("open"));
    vi.advanceTimersByTime(5000);
    expect(sources).toHaveLength(1);
    expect(get(trackUpdatesConnectionStatus)).toBe("connected");
  });
});
