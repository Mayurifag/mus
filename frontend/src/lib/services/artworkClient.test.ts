import { afterEach, describe, expect, it, vi } from "vitest";
import {
  searchArtwork,
  searchArtworkStream,
  preloadArtworkSearch,
} from "./artworkClient";
import type { ArtworkSearchResult } from "$lib/types";

const artwork: ArtworkSearchResult = {
  id: "itunes-1",
  source: "iTunes",
  title: "Песня",
  artist: "Artist",
  image_url: "https://example.com/cover.jpg",
  thumbnail_url: "https://example.com/small.jpg",
  width: 1200,
  height: 1200,
};

afterEach(() => vi.unstubAllGlobals());

describe("artwork client", () => {
  it("does not reuse another query's cache when a title or artist contains a pipe", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify([artwork])))
      .mockResolvedValueOnce(new Response("[]\n"));
    vi.stubGlobal("fetch", fetchMock);
    await searchArtwork({ title: "Pipe|Title", artist: "Artist" });
    await searchArtworkStream({
      title: "Pipe",
      artist: "Title|Artist",
      onResults: vi.fn(),
    });
    const onResults = vi.fn();
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("[]\n")));
    await searchArtworkStream({
      title: "Pipe|Title",
      artist: "Artist",
      onResults,
    });
    expect(onResults.mock.calls[0]).toEqual([[artwork]]);
  });

  it("decodes split UTF-8 records and a final record without a newline", async () => {
    const bytes = new TextEncoder().encode(
      `${JSON.stringify([artwork])}\n\n[]`,
    );
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        for (const byte of bytes) controller.enqueue(Uint8Array.of(byte));
        controller.close();
      },
    });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(body)));
    const onResults = vi.fn();
    await searchArtworkStream({
      title: "split stream",
      artist: "Artist",
      onResults,
    });
    expect(onResults.mock.calls).toEqual([[[artwork]], [[]]]);
    expect(body.locked).toBe(false);
  });

  it("reuses cached results before reading a refreshed stream", async () => {
    const query = { title: "cached artwork", artist: "Artist" };
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify([artwork])))
      .mockResolvedValueOnce(new Response("[]\n"));
    vi.stubGlobal("fetch", fetchMock);
    await searchArtwork(query);
    const cancel = preloadArtworkSearch(query);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    cancel();
    const onResults = vi.fn();
    await searchArtworkStream({ ...query, onResults });
    expect(onResults.mock.calls).toEqual([[[artwork]], [[]]]);
  });

  it("releases the stream reader when a record is invalid", async () => {
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("not json\n"));
        controller.close();
      },
    });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(body)));
    await expect(
      searchArtworkStream({
        title: "invalid stream",
        artist: "Artist",
        onResults: vi.fn(),
      }),
    ).rejects.toBeInstanceOf(SyntaxError);
    expect(body.locked).toBe(false);
  });

  it("cancels an unfinished stream when parsing fails", async () => {
    const cancel = vi.fn();
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("invalid json\n"));
      },
      cancel,
    });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(body)));
    await expect(
      searchArtworkStream({
        title: "unfinished stream",
        artist: "Artist",
        onResults: vi.fn(),
      }),
    ).rejects.toBeInstanceOf(SyntaxError);
    expect(cancel).toHaveBeenCalledOnce();
    expect(body.locked).toBe(false);
  });

  it("preserves a callback error if stream cleanup also fails", async () => {
    const error = new Error("Cannot display artwork");
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("[]\n"));
      },
      cancel() {
        throw new Error("Cancellation failed");
      },
    });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(body)));
    await expect(
      searchArtworkStream({
        title: "callback failure",
        artist: "Artist",
        onResults: () => {
          throw error;
        },
      }),
    ).rejects.toBe(error);
    expect(body.locked).toBe(false);
  });
});
