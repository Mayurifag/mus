/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

import { build, files, prerendered, version } from "$service-worker";

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

  if (request.method !== "GET" || request.url.startsWith("chrome-extension://")) {
    return;
  }

  const url = new URL(request.url);

  if (url.pathname.startsWith("/_app/immutable/")) {
    event.respondWith(
      caches.match(request).then(async (cachedResponse) => {
        if (cachedResponse) {
          return cachedResponse;
        }
        const networkResponse = await fetch(request);
        const cache = await caches.open(CACHE_NAME);
        cache.put(request, networkResponse.clone());
        return networkResponse;
      })
    );
    return;
  }

  event.respondWith(
    fetch(request)
      .then(async (networkResponse) => {
        const cache = await caches.open(CACHE_NAME);
        cache.put(request, networkResponse.clone());
        return networkResponse;
      })
      .catch(async () => {
        const cachedResponse = await caches.match(request);
        if (request.mode === 'navigate' && !cachedResponse) {
          const rootResponse = await caches.match('/');
          return rootResponse ?? new Response("You are currently offline.", {
            status: 503,
            headers: { "Content-Type": "text/plain" }
          });
        }
        return cachedResponse ?? new Response(null, { status: 404 });
      })
  );
});
