/// <reference types="@sveltejs/kit" />
/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />

import { build, files, version } from '$service-worker';

// Create a unique cache name for this deployment
const CACHE = `cache-${version}`;

// Define assets to precache
const ASSETS = [
	...build, // the app itself
	...files, // everything in `static`
	'/favicon.png',
	'/manifest.json'
];

// Add routes to cache for offline access
const ROUTES = ['/'];

const sw = self as unknown as ServiceWorkerGlobalScope;

sw.addEventListener('install', (event) => {
	// Create a new cache and add all files to it
	async function addFilesToCache() {
		const cache = await caches.open(CACHE);
		await cache.addAll(ASSETS);
		await cache.addAll(ROUTES);
	}

	event.waitUntil(addFilesToCache());
});

sw.addEventListener('activate', (event) => {
	// Remove previous cached data from disk
	async function deleteOldCaches() {
		for (const key of await caches.keys()) {
			if (key !== CACHE) await caches.delete(key);
		}
	}

	event.waitUntil(deleteOldCaches());
});

sw.addEventListener('fetch', (event) => {
	// ignore non-GET requests
	if (event.request.method !== 'GET') return;

	async function respond() {
		const url = new URL(event.request.url);
		const cache = await caches.open(CACHE);

		// For API requests, try network first
		if (url.pathname.startsWith('/api/')) {
			try {
				const response = await fetch(event.request);
				if (response.ok) {
					cache.put(event.request, response.clone());
				}
				return response;
			} catch (err) {
				const cachedResponse = await cache.match(event.request);
				if (cachedResponse) {
					return cachedResponse;
				}
				throw err;
			}
		}

		// For static assets, check cache first
		if (ASSETS.includes(url.pathname) || url.pathname.startsWith('/images/')) {
			const cachedResponse = await cache.match(url.pathname);
			if (cachedResponse) {
				return cachedResponse;
			}
		}

		// For everything else, try network first, fallback to cache
		try {
			const response = await fetch(event.request);
			if (response.status === 200) {
				cache.put(event.request, response.clone());
			}
			return response;
		} catch (err) {
			const cachedResponse = await cache.match(event.request);
			if (cachedResponse) {
				return cachedResponse;
			}
			// Fall back to app shell for navigation requests
			if (event.request.mode === 'navigate') {
				const homePageResponse = await cache.match('/');
				if (homePageResponse) {
					return homePageResponse;
				}
			}
			throw err;
		}
	}

	event.respondWith(respond());
});
