import { afterEach, describe, expect, it, vi } from "vitest";
import { API_BASE_URL } from "./apiBase";
import {
  deleteTrack,
  fetchTrackCutPreview,
  requeueTrack,
  updateTrack,
} from "./trackClient";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe.each([
  {
    name: "updateTrack",
    request: () => updateTrack("track", { title: "Song" }),
  },
  { name: "deleteTrack", request: () => deleteTrack("track") },
  { name: "requeueTrack", request: () => requeueTrack("track") },
  {
    name: "fetchTrackCutPreview",
    request: () =>
      fetchTrackCutPreview(
        "track",
        { start_seconds: 1, end_seconds: 2 },
        false,
      ),
  },
])("$name", ({ request }) => {
  it("preserves the server's error detail", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          new Response(
            JSON.stringify({ detail: "Music directory is read-only" }),
            { status: 403 },
          ),
        ),
    );
    await expect(request()).rejects.toThrow("Music directory is read-only");
  });
});

it("deletes a track without trying to parse an empty response", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response(null, { status: 202 })),
  );
  await expect(deleteTrack("track")).resolves.toBeUndefined();
});

it.each(["queued", "no_changes"] as const)(
  "accepts the server's %s edit response",
  async (status) => {
    const fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ status }), {
        headers: { "Content-Type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetch);
    const update = { title: "Song", tags: ["cover"] };

    await expect(updateTrack("track", update)).resolves.toEqual({ status });
    expect(fetch).toHaveBeenCalledWith(`${API_BASE_URL}/tracks/track`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(update),
    });
  },
);

it("returns preview audio without parsing it as JSON", async () => {
  const response = new Response("audio bytes", {
    headers: { "Content-Type": "audio/wav" },
  });
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response));
  const audio = await fetchTrackCutPreview(
    "track",
    { start_seconds: 1, end_seconds: 2 },
    false,
  );
  expect(await audio.text()).toBe("audio bytes");
});
