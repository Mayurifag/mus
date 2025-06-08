# Enhance Mobile Web and PWA Compliance

## User Problem
The music player app lacks full compliance with modern mobile web standards, specifically Media Session API for native controls and PWA (Progressive Web App) requirements, leading to a suboptimal user experience on mobile devices. The manifest colors are also inconsistent with the app theme.

## High-Level Solution
1.  Integrate the Media Session API to provide native media controls and metadata display on mobile platforms.
2.  Implement and validate PWA standards, including a complete `manifest.json`, a service worker for basic offline functionality, and proper app installation/display behaviors.
3.  Update `manifest.json` color values to match the application's theme (`#020817`).
4.  Conduct thorough cross-platform testing to ensure compliance and consistent behavior.

## Success Metrics
*   Media Session API correctly displays track metadata (title, artist, album, artwork) and handles play/pause, next/previous actions on Safari iOS, Android media notifications, and Chrome/Edge mobile.
*   App meets PWA installability criteria as verified by Lighthouse and manual testing (install prompts appear, app installs correctly).
*   Service worker successfully caches app shell (using SvelteKit's `$service-worker` module assets), allowing the app to load with basic functionality when offline.
*   `manifest.json` is valid, complete, and `theme_color` / `background_color` are set to `#020817`. Splash screens use these colors.
*   Splash screens are displayed correctly during PWA launch.
*   App functions consistently across Safari on iOS, Chrome/Firefox on Android, and desktop mobile simulations, for all implemented features.
*   `make ci` passes without errors after all changes.

## Detailed Description
This task involves a comprehensive update to the frontend application to ensure it adheres to modern mobile web standards, including robust Media Session API integration and full PWA compliance. The goal is to improve the user experience on mobile devices by enabling native media controls, offline capabilities, and app installability.

**Context:**
*   Frontend: SvelteKit and TypeScript, using Svelte 5 runes for reactivity.
*   Backend: FastAPI.
*   Current PWA status: A `manifest.json` and `app.html` exist but need review and enhancement. Service worker functionality needs to be implemented. SvelteKit's service worker integration (via `src/service-worker.ts` and `$service-worker` module) should be leveraged.

**Key Areas of Focus:**
1.  **Media Session API**: Implement metadata updates (title, artist, album, artwork) and action handlers (play, pause, nexttrack, previoustrack). Avoid seek-related actions due to iOS compatibility issues.
2.  **PWA Manifest (`frontend/static/manifest.json`)**: Ensure it's complete, valid, and uses the app's theme color (`#020817` for `theme_color` and `background_color`). Configure icons (ensure 192x192 and 512x512 SVG icons with `purpose: "any maskable"`), `start_url`, `display` mode (`standalone`), and `orientation` (`portrait-primary`). Add `scope: "/"` and `categories: ["music", "entertainment"]`.
3.  **Service Worker (`frontend/src/service-worker.ts`)**: Implement a service worker to cache the app shell for basic offline access using SvelteKit's `$service-worker` module. Justify why full music stream caching is not implemented as part of basic offline support.
4.  **App Shell (`frontend/src/app.html`)**: Ensure necessary meta tags for PWA behavior, including `theme-color`, `apple-mobile-web-app-capable`, and `apple-mobile-web-app-status-bar-style`.
5.  **App Installation & Display**: Verify app installation prompts and behavior, including splash screens.
6.  **Cross-Platform Testing**: Validate all changes on Safari iOS (iPhone/iPad), Chrome/Firefox on Android, and desktop browsers with mobile simulation.

## Implementation Considerations
*   For each subtask involving code changes, provide the **complete and updated content** of all modified files using Markdown code blocks with the file path in a comment on the first line. Do not use diffs or partial snippets.
*   Ensure all JavaScript/TypeScript code adheres to Svelte 5 runes and the project's linting/formatting standards as per `.augment-guidelines`.
*   Media Session artwork: The API expects an array of `MediaImage` objects. Provide multiple sizes using `track.cover_small_url` and `track.cover_original_url`. The `src` attribute must be a valid URL. Image type should be `image/webp` as per backend endpoint.
*   Service Worker: Use SvelteKit's `$service-worker` module (`build`, `files`, `prerendered`, `version`) to manage caching of app shell assets.
*   Verify manifest colors (`theme_color`, `background_color`) for accessibility and contrast, especially for splash screens and status bars.
*   The service worker should be registered in `frontend/src/routes/(app)/+layout.svelte` or handled automatically by SvelteKit if `src/service-worker.ts` is present.

## Subtasks

### [ ] 1. Implement Media Session API Integration
**Description**: Integrate the Media Session API for native media controls and metadata display.
**Details**:
    *   Modify `frontend/src/lib/services/AudioService.ts` to manage Media Session API interactions.
    *   When a track starts playing or changes, or playback state changes:
        *   Update `navigator.mediaSession.metadata` with:
            *   `title`: Current track's `title`.
            *   `artist`: Current track's `artist`.
            *   `album`: Current track's `artist` (if no distinct album field).
            *   `artwork`: An array of `MediaImage` objects. Use `track.cover_small_url` and `track.cover_original_url` if available.
                Example:
                ```javascript
                const artwork = [];
                if (track.cover_small_url) {
                  artwork.push({ src: track.cover_small_url, sizes: '192x192', type: 'image/webp' });
                }
                if (track.cover_original_url) {
                  artwork.push({ src: track.cover_original_url, sizes: '512x512', type: 'image/webp' });
                }
                navigator.mediaSession.metadata = new MediaMetadata({
                  title: track.title,
                  artist: track.artist,
                  album: track.artist, // Using artist as album
                  artwork: artwork
                });
                ```
        *   Update `navigator.mediaSession.playbackState` to `'playing'` or `'paused'` based on `audioService.isPlaying`.
    *   Set up action handlers using `navigator.mediaSession.setActionHandler()` within `AudioService.ts`, triggered when a track is loaded or playback state changes:
        *   `'play'`: Call `this.play()` (within `AudioService`).
        *   `'pause'`: Call `this.pause()` (within `AudioService`).
        *   `'previoustrack'`: Call `trackStore.previousTrack()`. Ensure `trackStore` is accessible or an event/callback mechanism is used if direct import is problematic.
        *   `'nexttrack'`: Call `trackStore.nextTrack()`.
    *   DO NOT implement handlers for `seekforward`, `seekbackward`, or `seekto`.
    *   These updates should primarily occur within `AudioService.ts` when its internal state (current track, playback status) changes.
    *   Verify that metadata and controls appear correctly on Safari iOS (lock screen, control center), Android media notifications, and desktop Chrome/Edge mobile views.
    *   Provide the complete updated content for all modified files.
**Filepaths to Modify**: `frontend/src/lib/services/AudioService.ts`
**Relevant Make Commands (Optional)**: `make front-lint, make front-svelte-check`

### [ ] 2. Configure PWA Manifest and Core App Shell Settings
**Description**: Update `manifest.json` for PWA compliance, ensure `app.html` has necessary tags, and configure PWA display properties.
**Details**:
    *   **`frontend/static/manifest.json`**:
        *   Update the following fields to ensure compliance and consistency:
            *   `name`: "Mus - Personal Music Player"
            *   `short_name`: "Mus"
            *   `description`: "Your personal music player, always with you."
            *   `icons`: Ensure 192x192 and 512x512 SVG icons are present, and set `purpose` to `"any maskable"`.
                Example:
                ```json
                "icons": [
                  {
                    "src": "/images/icon-192.svg",
                    "sizes": "192x192",
                    "type": "image/svg+xml",
                    "purpose": "any maskable"
                  },
                  {
                    "src": "/images/icon-512.svg",
                    "sizes": "512x512",
                    "type": "image/svg+xml",
                    "purpose": "any maskable"
                  }
                ]
                ```
            *   `start_url`: "/"
            *   `display`: "standalone"
            *   `background_color`: "#020817"
            *   `theme_color`: "#020817"
            *   `orientation`: "portrait-primary"
            *   `scope`: "/"
            *   `categories`: `["music", "entertainment"]`
    *   **`frontend/src/app.html`**:
        *   Ensure `<meta name="theme-color" content="#020817" />` is present and correct.
        *   Ensure `<link rel="manifest" href="%sveltekit.assets%/manifest.json" />` is correct.
        *   Ensure `<link rel="apple-touch-icon" href="%sveltekit.assets%/images/icon-192.svg" />` (or a suitable high-res icon) is present.
        *   Add `<meta name="apple-mobile-web-app-capable" content="yes" />`.
        *   Add `<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />` for better dark theme integration on iOS.
    *   Verify splash screen behavior on install (driven by manifest icons and `background_color`).
    *   Test PWA installability using Lighthouse in Chrome DevTools and manual "Add to Home Screen" on iOS and Android.
    *   Provide the complete updated content for all modified files.
**Filepaths to Modify**: `frontend/static/manifest.json`, `frontend/src/app.html`
**Relevant Make Commands (Optional)**: `make front-lint`

### [ ] 3. Implement Service Worker for Basic Offline Support
**Description**: Create and register a service worker to cache the application shell for basic offline functionality.
**Details**:
    *   Create a new service worker file: `frontend/src/service-worker.ts`.
    *   In `frontend/src/service-worker.ts`:
        *   Import SvelteKit service worker utilities: `import { build, files, prerendered, version } from '$service-worker';`
        *   Define a unique cache name using the `version` string: `const CACHE_NAME = \`mus-cache-\${version}\`;`
        *   List app shell assets to cache by combining `build`, `files`, and `prerendered` arrays. Ensure essential assets like `/`, `/manifest.json`, and icons specified in `files` (if not already in `build`) are included.
            ```typescript
            const ASSETS_TO_CACHE = [...build, ...files, ...prerendered];
            ```
        *   Implement `install` event listener: Open the cache (using `CACHE_NAME`) and add all `ASSETS_TO_CACHE`.
            ```typescript
            self.addEventListener('install', (event) => {
              event.waitUntil(
                caches.open(CACHE_NAME)
                  .then((cache) => cache.addAll(ASSETS_TO_CACHE))
                  .then(() => (self as unknown as ServiceWorkerGlobalScope).skipWaiting())
              );
            });
            ```
        *   Implement `activate` event listener: Clean up old caches. Iterate through `caches.keys()` and delete any cache not matching `CACHE_NAME`. Ensure client claims.
            ```typescript
            self.addEventListener('activate', (event) => {
              event.waitUntil(
                caches.keys().then(async (keys) => {
                  for (const key of keys) {
                    if (key !== CACHE_NAME) {
                      await caches.delete(key);
                    }
                  }
                  (self as unknown as ServiceWorkerGlobalScope).clients.claim();
                })
              );
            });
            ```
        *   Implement `fetch` event listener:
            *   For navigation requests (HTML pages, `event.request.mode === 'navigate'`): Attempt a network-first strategy. If network fails, try to serve `/` (or a specific offline page if designed) from cache.
            *   For other assets (CSS, JS, images identified in `ASSETS_TO_CACHE`): Use a cache-first strategy. Try to serve from cache; if not found, fetch from network, cache it (if appropriate), and then serve.
            *   **Do NOT cache music streams (`/api/v1/tracks/.../stream`) by default.**
                *   **Justification**: Caching full music streams for basic offline support is not recommended due to:
                    1.  **Storage Quotas**: Audio files are large and can quickly exceed browser storage limits.
                    2.  **User Experience**: Consuming significant storage and potentially mobile data without explicit user consent or control is undesirable.
                    3.  **Complexity**: Managing a large, dynamic cache of media files with updates and eviction policies is complex for a basic offline feature.
                    4.  **PWA Philosophy**: The primary goal for basic PWA offline is reliable app shell loading. Extensive data caching is typically an explicit, user-driven feature.
    *   **Service Worker Registration**:
        *   SvelteKit automatically discovers and registers `src/service-worker.ts`. No manual registration in `+layout.svelte` is typically needed if this naming convention is followed. Ensure this is the case or provide registration code if SvelteKit's auto-registration is not active/configured. If manual registration is chosen for `frontend/src/routes/(app)/+layout.svelte`:
            ```typescript
            // In onMount within +layout.svelte
            if ('serviceWorker' in navigator && browser && (import.meta.env.PROD || import.meta.env.VITE_ENABLE_SW === 'true')) {
              navigator.serviceWorker.register('/service-worker.js') // Path SvelteKit makes it available at
                .then(registration => console.log('Service Worker registered with scope:', registration.scope))
                .catch(error => console.error('Service Worker registration failed:', error));
            }
            ```
    *   Test offline functionality: Launch the app, go offline (Chrome DevTools network tab), and try to reload/navigate. The basic shell should load.
    *   Provide the complete updated content for all modified files.
**Filepaths to Modify**: Create `frontend/src/service-worker.ts`. Potentially `frontend/src/routes/(app)/+layout.svelte` if manual registration is chosen over SvelteKit's default.
**Relevant Make Commands (Optional)**: `make front-build, make front-lint`

### [ ] 4. Cross-Platform Testing and Refinement
**Description**: Systematically test all implemented Media Session API and PWA features across target platforms and browsers, addressing any bugs or inconsistencies.
**Details**:
    *   **Media Session API Testing**:
        *   Safari on iOS (iPhone/iPad):
            *   Verify lock screen controls (play/pause, next/previous, artwork, title, artist, album).
            *   Verify Control Center media widget.
            *   Verify Notification Center media widget.
        *   Chrome/Firefox on Android:
            *   Verify media notification controls and metadata.
        *   Desktop Chrome/Edge (using mobile simulation and media keys):
            *   Verify media session integration.
    *   **PWA Features Testing**:
        *   Safari on iOS:
            *   "Add to Home Screen" functionality.
            *   App launch from home screen (splash screen, standalone mode, theme colors).
            *   Offline loading of the app shell.
        *   Chrome/Firefox on Android:
            *   Installation prompt.
            *   "Add to Home Screen" / Install app functionality.
            *   App launch from home screen (splash screen, standalone mode, theme colors).
            *   Offline loading of the app shell.
        *   Desktop Chrome/Edge:
            *   Installability and PWA behavior.
            *   Lighthouse PWA audit: Check for full PWA compliance, including manifest validity, SW registration, HTTPS, installability criteria.
    *   **General Testing**:
        *   Test across various screen sizes and orientations (especially `portrait-primary` enforcement).
        *   Verify `theme-color` and `background_color` in manifest are correctly applied to browser UI elements (e.g., status bar on Android, address bar theming).
        *   Ensure color accessibility and contrast ratios for manifest-driven UI elements are acceptable.
    *   Identify and fix any bugs, visual glitches, or behavioral inconsistencies found during testing.
    *   Provide the complete updated content for any files modified during this refinement phase.
**Filepaths to Modify**: Any files from Subtasks 1, 2, or 3 that require adjustments based on testing.
**Relevant Make Commands (Optional)**: `make front-dev, make ci`
