const CACHE_NAME = 'mus-v1'
const STATIC_ASSETS = [
  '/',
  '/static/css/style.css',
  '/static/js/main.js',
  '/static/js/audioManager.js',
  '/static/js/trackManager.js',
  '/static/js/uiControls.js',
  '/static/js/stateManager.js',
  '/static/js/volume.js',
  '/static/js/mediaSessionManager.js',
  '/static/manifest.json',
  '/static/images/placeholder.svg',
  '/static/android-chrome-512x512.png',
  '/static/android-chrome-192x192.png'
]

self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => cache.addAll(STATIC_ASSETS))
      .then(() => self.skipWaiting())
  )
})

self.addEventListener('activate', event => {
  event.waitUntil(
    Promise.all([
      self.clients.claim(),
      caches.keys().then(cacheNames => {
        return Promise.all(
          cacheNames
            .filter(cacheName => cacheName !== CACHE_NAME)
            .map(cacheName => caches.delete(cacheName))
        )
      })
    ])
  )
})

self.addEventListener('fetch', event => {
  if (event.request.method !== 'GET') {
    return
  }

  event.respondWith(
    caches.match(event.request)
      .then(response => {
        if (response) {
          return response
        }
        return fetch(event.request)
      })
  )
})
