import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("$app/env", () => ({ version: "test" }));
vi.mock("$app/manifest", () => ({
  immutable: [],
  assets: [],
  prerendered: [],
}));

interface WorkerEvent {
  request: Request;
  waitUntil: (promise: Promise<unknown>) => void;
  respondWith: (promise: Promise<Response>) => void;
}

const listeners = new Map<string, (event: WorkerEvent) => void>();
const cache = {
  match: vi.fn(),
  put: vi.fn().mockResolvedValue(undefined),
  addAll: vi.fn().mockResolvedValue(undefined),
};
const cacheStorage = {
  keys: vi.fn(),
  delete: vi.fn().mockResolvedValue(true),
  open: vi.fn().mockResolvedValue(cache),
  match: vi.fn(),
};

beforeEach(async () => {
  vi.resetModules();
  vi.clearAllMocks();
  cache.match.mockResolvedValue(undefined);
  cacheStorage.match.mockResolvedValue(undefined);
  listeners.clear();
  vi.stubGlobal("self", {
    location: new URL("https://mus.test/service-worker.js"),
    addEventListener: (type: string, listener: (event: WorkerEvent) => void) =>
      listeners.set(type, listener),
    clients: { claim: vi.fn().mockResolvedValue(undefined) },
  });
  vi.stubGlobal("caches", cacheStorage);
  await import("../../service-worker/index");
});

afterEach(() => vi.unstubAllGlobals());

function dispatch(type: string, request = new Request("https://mus.test/")) {
  const promises: Promise<unknown>[] = [];
  let response: Promise<Response> | undefined;
  listeners.get(type)!({
    request,
    waitUntil: (promise) => promises.push(promise),
    respondWith: (promise) => {
      response = promise;
    },
  });
  return { response, done: () => Promise.all(promises) };
}

describe("service worker cache lifecycle", () => {
  it("removes only previous mus caches during activation", async () => {
    cacheStorage.keys.mockResolvedValue([
      "mus-cache-old",
      "mus-cache-test",
      "other-app-offline-data",
    ]);
    await dispatch("activate").done();
    expect(cacheStorage.delete.mock.calls).toEqual([["mus-cache-old"]]);
  });

  it("does not persist missing chunks, so a later request can recover", async () => {
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(new Response("missing", { status: 404 }))
      .mockResolvedValueOnce(new Response("chunk"));
    vi.stubGlobal("fetch", fetch);
    const request = new Request(
      "https://mus.test/_app/immutable/chunks/app.js",
    );
    const missing = dispatch("fetch", request);
    expect((await missing.response)?.status).toBe(404);
    await missing.done();
    expect(cache.put).not.toHaveBeenCalled();

    const recovered = dispatch("fetch", request);
    expect(await (await recovered.response)?.text()).toBe("chunk");
    await recovered.done();
    expect(cache.put).toHaveBeenCalledOnce();
  });

  it("reads assets from this version's cache, not unrelated caches", async () => {
    cacheStorage.match.mockResolvedValue(new Response("unrelated"));
    cache.match.mockResolvedValue(new Response("current"));
    const event = dispatch(
      "fetch",
      new Request("https://mus.test/_app/immutable/chunks/app.js"),
    );
    expect(await (await event.response)?.text()).toBe("current");
    expect(cacheStorage.match).not.toHaveBeenCalled();
  });

  it("returns network data even when cache storage is full", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("chunk")));
    cache.put.mockRejectedValueOnce(new Error("Quota exceeded"));
    const log = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      const event = dispatch(
        "fetch",
        new Request("https://mus.test/_app/immutable/chunks/app.js"),
      );
      expect(await (await event.response)?.text()).toBe("chunk");
      await event.done();
      expect(log).toHaveBeenCalled();
    } finally {
      log.mockRestore();
    }
  });
});
