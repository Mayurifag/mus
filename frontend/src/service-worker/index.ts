/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

import { version } from "$app/env";
import { immutable, assets, prerendered } from "$app/manifest";
import { isStaticAsset, shouldSkipCache } from "$lib/utils/serviceWorkerCache";

declare const self: ServiceWorkerGlobalScope;

const CACHE_PREFIX = "mus-cache-";
const CACHE_NAME = `${CACHE_PREFIX}${version}`;
const ASSETS_TO_CACHE = [...immutable, ...assets, ...prerendered].map(
  (asset) => new URL(asset.path, self.location.href).pathname,
);

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
      if (key.startsWith(CACHE_PREFIX) && key !== CACHE_NAME) {
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

  if (isStaticAsset(url.pathname)) {
    event.respondWith(
      caches.open(CACHE_NAME).then(async (cache) => {
        const cachedResponse = await cache.match(request);
        if (cachedResponse) {
          return cachedResponse;
        }
        const networkResponse = await fetch(request);
        if (networkResponse.ok) {
          try {
            await cache.put(request, networkResponse.clone());
          } catch (error) {
            console.error("[SW] Failed to cache asset:", error);
          }
        }
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
        const cache = await caches.open(CACHE_NAME);
        const rootResponse = await cache.match("/");
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
