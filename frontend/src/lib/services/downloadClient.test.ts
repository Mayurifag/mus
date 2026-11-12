import { afterEach, describe, expect, it, vi } from "vitest";
import { confirmDownload, startDownload } from "./downloadClient";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe.each([
  {
    name: "startDownload",
    request: () => startDownload("https://example.com/video"),
  },
  {
    name: "confirmDownload",
    request: () =>
      confirmDownload("https://example.com/video", "Song", "Artist"),
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
            JSON.stringify({ detail: "A file with this name already exists" }),
            { status: 409 },
          ),
        ),
    );
    await expect(request()).rejects.toThrow(
      "A file with this name already exists",
    );
  });

  it("preserves the network error", async () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    const error = new TypeError("Network unavailable");
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(error));
    await expect(request()).rejects.toBe(error);
  });

  it("accepts a queued download response", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          new Response(JSON.stringify({ status: "accepted" }), { status: 202 }),
        ),
    );
    await expect(request()).resolves.toBeUndefined();
  });
});
