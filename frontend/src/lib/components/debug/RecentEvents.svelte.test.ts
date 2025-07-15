import { describe, it, expect, beforeEach } from "vitest";
import { render } from "@testing-library/svelte";
import RecentEvents from "./RecentEvents.svelte";
import { recentEventsStore } from "$lib/stores/recentEventsStore";
import type { MusEvent } from "$lib/types";

describe("RecentEvents", () => {
  beforeEach(() => {
    recentEventsStore.clear();
  });

  it("should render nothing when no events exist", () => {
    const { container } = render(RecentEvents);
    expect(container.querySelector(".border-t")).toBeNull();
  });

  it("should render events when they exist", () => {
    const testEvent: MusEvent = {
      message_to_show: "Test event message",
      message_level: "success",
      action_key: "track_added",
      action_payload: null,
    };

    recentEventsStore.addEvent(testEvent);

    const { getByText } = render(RecentEvents);
    expect(getByText("Recent Events")).toBeInTheDocument();
    expect(getByText("Test event message")).toBeInTheDocument();
  });

  it("should display timestamp in HH:MM:SS format", () => {
    const testEvent: MusEvent = {
      message_to_show: "Test event",
      message_level: "info",
      action_key: null,
      action_payload: null,
    };

    recentEventsStore.addEvent(testEvent);

    const { container } = render(RecentEvents);
    const timeElement = container.querySelector(".text-muted-foreground\\/50");
    expect(timeElement?.textContent).toMatch(/^\d{2}:\d{2}:\d{2}$/);
  });

  it("should show action_key when no message is provided", () => {
    const testEvent: MusEvent = {
      message_to_show: null,
      message_level: "info",
      action_key: "track_updated",
      action_payload: null,
    };

    recentEventsStore.addEvent(testEvent);

    const { getByText } = render(RecentEvents);
    expect(getByText("track_updated")).toBeInTheDocument();
  });

  it("should show 'Unknown event' when neither message nor action_key is provided", () => {
    const testEvent: MusEvent = {
      message_to_show: null,
      message_level: "info",
      action_key: null,
      action_payload: null,
    };

    recentEventsStore.addEvent(testEvent);

    const { getByText } = render(RecentEvents);
    expect(getByText("Unknown event")).toBeInTheDocument();
  });
});
