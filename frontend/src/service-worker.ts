/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

import { build, files, prerendered, version } from "$service-worker";

declare const self: ServiceWorkerGlobalScope;

const CACHE_NAME = `mus-cache-${version}`;
const ASSETS_TO_CACHE = [...build, ...files, ...prerendered];

self.addEventListener("install", (event) => {
  async function addAllToCache() {
    const cache = await caches.open(CACHE_NAME);
    const cachePromises = ASSETS_TO_CACHE.map((assetUrl) => {
      return cache.add(assetUrl).catch((reason) => {
        console.error(`[SW] Failed to cache ${assetUrl}:`, reason);
      });
    });
    await Promise.all(cachePromises);
  }

  event.waitUntil(addAllToCache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then(async (keys) => {
      for (const key of keys) {
        if (key !== CACHE_NAME) {
          await caches.delete(key);
        }
      }
      self.clients.claim();
    }),
  );
});

self.addEventListener("fetch", (event) => {
  const { request } = event;

  if (request.method !== "GET") {
    return;
  }

  const url = new URL(request.url);
  const isStaticAsset = ASSETS_TO_CACHE.includes(url.pathname);

  if (isStaticAsset) {
    event.respondWith(
      caches.match(request).then((cachedResponse) => {
        return (
          cachedResponse ||
          fetch(request).then((networkResponse) => {
            return networkResponse;
          })
        );
      }),
    );
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request).catch(async () => {
        const cachedResponse = await caches.match("/");
        return (
          cachedResponse ||
          new Response("You are currently offline.", { status: 503 })
        );
      }),
    );
    return;
  }
});
