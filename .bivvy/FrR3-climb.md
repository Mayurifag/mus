<Climb>
  <header>
    <id>FrR3</id>
    <type>feature</type>
    <description>Complete frontend redesign and rebuild with SvelteKit, Shadcn-Svelte, Tailwind CSS, xior. Features: persistent player state (shuffle/repeat), dynamic UI updates via SSE from backend periodic background scan, visual volume feedback. Scan button/endpoint removed from backend (scan button on frontend page uses existing periodic scan, manual trigger removed).</description>
  </header>
  <newDependencies>
    - Frontend: `xior` (runtime), `@vitest/coverage-v8` (devDependency).
    - Backend: (Implicitly, `is_shuffle` and `is_repeat` added to `PlayerState` entity/DTO, already implemented).
  </newDependencies>
  <prerequisitChanges>
    - Existing SvelteKit frontend structure (`frontend/`) will be heavily refactored. Native `fetch` in `apiClient.ts` will be replaced with `xior`.
    - Existing backend API endpoints for tracks and player state are functional. The manual `POST /api/v1/scan` endpoint has been removed from the backend.
    - Backend `PlayerState` entity and DTO now support `current_track_id`, `progress_seconds`, `volume_level`, `is_muted`, `is_shuffle: boolean`, `is_repeat: boolean`.
    - Backend `lifespan` event modified: DB reset and cover cleaning retained as startup. Periodic background scanning (`PeriodicScanner`) and SSE mechanism for updates are initiated. `ScanTracksUseCase` invoked by this periodic task.
  </prerequisitChanges>
  <relevantFiles>
    - `frontend/`: Entire directory.
        - `frontend/src/app.html` (PWA setup - done)
        - `frontend/src/app.css` (Global styles, Tailwind directives, CSS variables for theme - review if Tailwind fix needed)
        - `frontend/src/routes/(app)/+layout.svelte` (Main app layout, audio element, global state init, SSE client via eventHandlerService, shuffle/repeat logic, persistence)
        - `frontend/src/routes/(app)/+layout.server.ts` (Initial data loading - done)
        - `frontend/src/routes/(app)/+page.svelte` (Main page content, track list display - manual scan button removed)
        - `frontend/src/lib/components/layout/PlayerFooter.svelte` (volume feedback, shuffle/repeat buttons)
        - `frontend/src/lib/components/layout/RightSidebar.svelte` (New component for layout structure)
        - `frontend/src/lib/components/domain/TrackItem.svelte` (Interactive progress bar)
        - `frontend/src/lib/components/domain/TrackList.svelte` (Scroll to active)
        - `frontend/src/lib/stores/playerStore.ts` (shuffle/repeat logic, state persistence for new fields)
        - `frontend/src/lib/stores/trackStore.ts` (dynamic updates from SSE via eventHandlerService)
        - `frontend/src/lib/services/apiClient.ts` (xior refactor, SSE client logic)
        - `frontend/src/lib/services/eventHandlerService.ts` (New service for SSE event processing, toasts, and track updates)
        - `frontend/static/manifest.json`, `frontend/src/service-worker.ts` (PWA - done)
        - `frontend/package.json`, `frontend/vite.config.ts` (Add xior, coverage tool, coverage script)
        - `frontend/tailwind.config.js` (Review if Tailwind fix needed)
    - `backend/src/mus/main.py` (Lifespan, PeriodicScanner, SSE router - done)
    - `backend/src/mus/application/use_cases/scan_tracks_use_case.py` (Called by periodic task, generates generic SSE event payload)
    - `backend/src/mus/domain/entities/player_state.py` (add `is_shuffle`, `is_repeat` - done)
    - `backend/src/mus/application/dtos/player_state.py` (add `is_shuffle`, `is_repeat` - done)
    - `backend/src/mus/infrastructure/persistence/sqlite_player_state_repository.py` (Updated - done)
    - `backend/src/mus/infrastructure/tasks/background_scanner.py` (PeriodicScanner class, triggers SSE broadcast - done)
    - `backend/src/mus/infrastructure/api/sse_handler.py` (SSE logic and endpoint, sends flat JSON payload)
    - `backend/tests/api/test_sse_handler.py` (New test file for SSE functionality - to be created)
    - `backend/tests/infrastructure/tasks/test_background_scanner.py` (Tests for PeriodicScanner - done)
    - `backend/tests/test_main_startup.py` (Updated for PeriodicScanner task - done)
    - `backend/tests/application/test_scan_tracks_use_case.py` (Updated for generic SSE event payload generation)
  </relevantFiles>
  <everythingElse>
    ## Feature Overview

    *   **Feature Name and ID:** Frontend Redesign & Rebuild v4 (FrR3)
    *   **Purpose Statement:** To comprehensively overhaul the Mus music player's frontend, implementing a new UI/UX. This involves refactoring the SvelteKit application, integrating Shadcn-Svelte, Tailwind CSS, and `xior` for API calls (encapsulated in `apiClient.ts`). Key features include persistent player state (track ID, progress, volume, mute, shuffle, single-track repeat), dynamic tab titles, and dynamic track list updates via Server-Sent Events (SSE) triggered by a backend *periodic background scan (every minute)*. The manual "Scan Library" button and its API endpoint are removed from backend and frontend.
    *   **Problem Being Solved:**
        *   Suboptimal current frontend UI/UX and potential rendering issues.
        *   Lack of player state persistence and robust shuffle/repeat functionalities.
        *   Reliance on manual scans or a single startup scan, and full page reloads for library updates. Transitioning to automatic, periodic background updates.
        *   Backend scanning logic being refactored and encapsulated.
    *   **Success Metrics:**
        *   Frontend UI matches the specification (desktop/mobile), built with Tailwind utilities and Shadcn components minimizing custom CSS. Manual scan button removed from `+page.svelte`.
        *   Player state (including `is_shuffle`, `is_repeat`) saved to backend and correctly restored, with track list scrolling. Default: first track paused if no valid state. `playerStore` uses full `Track` object for UI, but only `track_id` is exchanged with backend.
        *   Dynamic tab title ("Artist - Track Name" if `$playerStore.currentTrack` is set, else "Mus").
        *   Backend performs a *periodic background scan* using `PeriodicScanner`; frontend `eventHandlerService.ts` receives generic SSE notifications (flat JSON payload with `message_to_show`, `message_level`, `action_key`) and dynamically updates track list via `apiClient.fetchTracks()` and `trackStore` without page reload. Toasts with appropriate messages (`New track: Artist - Title`, `X new tracks added`, `Y tracks updated`, or `Music library updated`) are shown based on scan results.
        *   `xior` is used for all API calls, strictly encapsulated within `apiClient.ts`. PWA functional.
        *   Shuffle mode (simple random next, history for previous - fallback to original previous) and Repeat Single mode implemented. Default behavior: loop all.
        *   Visual feedback for volume changes implemented.
        *   Code is "ideal," well-tested (Vitest, `vitest-coverage-v8`), and maintainable.
        *   Backend worker code for periodic scanning is refactored for clarity, testability, and reduced LOC in `main.py`.

    ## Requirements

    #### Functional Requirements (Core UI as per original spec, plus)

    1.  **UI Implementation (as per Spec - summarized):**
        *   Desktop: 2-column main area (Track List, Right Sidebar) above full-width Footer Player. Minimal custom CSS.
        *   Mobile: 1-column, Sidebar in `Sheet`.
        *   Track Item: Thumbnail, Name (consistent size), Artist. Playing: interactive progress bar, highlight.
        *   Footer Player: Track info, Progress Slider, Controls (Prev, Play/Pause, Next, Volume, Mute, Shuffle, Repeat). Buttons grouped logically.
        *   Styling: Minimal 3-color dark theme, `lucide-svelte` icons, `svelte-sonner` for toasts.

    2.  **Player State Persistence & Restoration (Revised):**
        *   **State:** `current_track_id` (sent/received), `progress_seconds`, `volume_level`, `is_muted`, `is_shuffle: boolean`, `is_repeat: boolean` (for single track repeat).
        *   **Save Triggers:** Page unload (`navigator.sendBeacon`), other changes (debounced `xior` call).
        *   **Restore/Init:** Restore state, scroll to track; else first track paused.

    3.  **Dynamic Tab Title (Revised):** "Artist - Track Name" if `$playerStore.currentTrack` is set, else "Mus".
    4.  **Automatic *Periodic Background Scan* & Dynamic Frontend Update (Revised):**
        *   **Backend:** On startup (`lifespan` event), backend *retains DB reset and cover cleaning operations*. It then instantiates and starts a `PeriodicScanner` background task. This task will invoke the `ScanTracksUseCase` to scan the music directory (e.g., every minute) and process tracks asynchronously.
        *   **SSE Notification:** After `ScanTracksUseCase` processes tracks, it triggers the sending of an SSE event (flat JSON payload with `message_to_show`, `message_level`, `action_key`) to all connected clients.
        *   **Frontend:** Listens to SSE via `eventHandlerService.ts` (initialized in `+layout.svelte`). On receiving relevant events with `action_key: 'reload_tracks'`, it calls `apiClient.fetchTracks()` and updates `trackStore` reactively. User-facing messages from the event are shown as toasts via `svelte-sonner`.
        *   `frontend/src/routes/(app)/+page.svelte` no longer contains a manual scan button or related logic.

    5.  **Shuffle and Repeat (Revised):**
        *   **Shuffle (`isShuffle`):** `nextTrack` picks random from full list (not current). `previousTrack` uses play history; if empty, plays previous in original order.
        *   **Repeat (`is_repeat` for single track):** OFF (Default): Loop entire sequence. ON: Current track loops.
    6.  **Visual Volume Feedback:** Temporary numeric percentage display on volume change.

    #### Technical Requirements (Core as per original spec, plus)

    *   **API Client Encapsulation**: `xior` usage strictly within `frontend/src/lib/services/apiClient.ts`.
    *   **Backend `PlayerState` Update:** Entity, DTO, repository, API to include `is_shuffle` and `is_repeat` (both booleans, defaulting to `False`) - Done.
    *   **Backend SSE Endpoint**: Existing endpoint `GET /api/v1/events/track-updates` for pushing notifications - Done (payload structure refactored to flat JSON).
    *   **Backend Periodic Background Worker**: `PeriodicScanner` class manages periodic scanning logic, initiated at startup - Done.

    ## Future Considerations (Post-Epic)
    *   Keyboard Shortcuts.
    *   Continuous runtime file system watching.
    *   Simple queue/playlist functionality.

  </everythingElse>
</Climb>
