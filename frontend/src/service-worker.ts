/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

import { build, files, prerendered, version } from "$service-worker";
import {
  isAudioStream,
  isStaticAsset,
  shouldSkipCache,
} from "$lib/utils/serviceWorkerCache";

declare const self: ServiceWorkerGlobalScope;

const CACHE_NAME = `mus-cache-${version}`;
const ASSETS_TO_CACHE = [...build, ...files, ...prerendered];

self.addEventListener("install", (event) => {
  async function addAllToCache() {
    const cache = await caches.open(CACHE_NAME);
    await cache.addAll(ASSETS_TO_CACHE).catch((reason) => {
      console.error(`[SW] Failed to cache assets:`, reason);
    });
  }

  event.waitUntil(addAllToCache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  async function deleteOldCaches() {
    const keys = await caches.keys();
    for (const key of keys) {
      if (key !== CACHE_NAME) {
        await caches.delete(key);
      }
    }
  }

  event.waitUntil(deleteOldCaches().then(() => self.clients.claim()));
});

self.addEventListener("fetch", (event) => {
  const { request } = event;

  if (
    request.method !== "GET" ||
    request.url.startsWith("chrome-extension://")
  ) {
    return;
  }

  const url = new URL(request.url);

  if (isAudioStream(url.pathname) && !request.headers.has("range")) {
    const headers = new Headers(request.headers);
    headers.set("Range", "bytes=0-");
    event.respondWith(fetch(new Request(request, { headers })));
    return;
  }

  if (isStaticAsset(url.pathname)) {
    event.respondWith(
      caches.match(request).then(async (cachedResponse) => {
        if (cachedResponse) {
          return cachedResponse;
        }
        const networkResponse = await fetch(request);
        const cache = await caches.open(CACHE_NAME);
        cache.put(request, networkResponse.clone());
        return networkResponse;
      }),
    );
    return;
  }

  if (shouldSkipCache(url.pathname)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request).catch(async () => {
        const rootResponse = await caches.match("/");
        return (
          rootResponse ??
          new Response("You are currently offline.", {
            status: 503,
            headers: { "Content-Type": "text/plain" },
          })
        );
      }),
    );
  }
});
