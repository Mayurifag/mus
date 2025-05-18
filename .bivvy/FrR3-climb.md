<Climb>
  <header>
    <id>FrR3</id>
    <type>feature</type>
    <description>Complete frontend redesign and rebuild with SvelteKit, Shadcn-Svelte, Tailwind CSS, xior. Features: persistent player state (shuffle/repeat), dynamic UI updates via SSE from backend periodic background scan, visual volume feedback. Scan button/endpoint removed.</description>
  </header>
  <newDependencies>
    - Frontend: `xior` (runtime), `@vitest/coverage-v8` (devDependency).
    - Backend: (Implicitly, `is_shuffle` and `is_repeat` added to `PlayerState` entity/DTO).
  </newDependencies>
  <prerequisitChanges>
    - Existing SvelteKit frontend structure (`frontend/`) will be heavily refactored.
    - Existing backend API endpoints for tracks and player state are functional. The manual `POST /api/v1/scan` endpoint is to be removed.
    - Backend `PlayerState` entity and DTO must support `current_track_id`, `progress_seconds`, `volume_level`, `is_muted`. This epic adds `is_shuffle: boolean` and `is_repeat: boolean` (for single track repeat).
    - Backend `lifespan` event needs modification: DB reset and cover cleaning are retained as startup operations. The initial startup scan is removed. A periodic background scanning task (managed by a new `PeriodicScanner` class) and an SSE mechanism for updates will be initiated instead. The `ScanTracksUseCase` will be invoked by this periodic task.
  </prerequisitChanges>
  <relevantFiles>
    - `frontend/`: Entire directory.
        - `frontend/src/app.html` (PWA setup)
        - `frontend/src/app.css` (Global styles, Tailwind directives, CSS variables for theme)
        - `frontend/src/routes/(app)/+layout.svelte` (Main app layout, audio element, global state init, SSE client)
        - `frontend/src/routes/(app)/+layout.server.ts` (Initial data loading)
        - `frontend/src/routes/(app)/+page.svelte` (Main page content, track list display)
        - `frontend/src/lib/components/layout/PlayerFooter.svelte` (volume feedback, shuffle/repeat buttons)
        - `frontend/src/lib/components/layout/RightSidebar.svelte` (Scan button removed, content might be just text or placeholder)
        - `frontend/src/lib/components/domain/TrackItem.svelte`
        - `frontend/src/lib/components/domain/TrackList.svelte`
        - `frontend/src/lib/stores/playerStore.ts` (shuffle/repeat logic, state persistence)
        - `frontend/src/lib/stores/trackStore.ts` (shuffle/repeat navigation logic, dynamic updates from SSE)
        - `frontend/src/lib/services/apiClient.ts` (xior refactor, SSE client logic, removed `triggerScan`)
        - `frontend/static/manifest.json`, `frontend/src/service-worker.ts` (PWA)
        - `frontend/package.json`, `frontend/vite.config.ts`, `frontend/tailwind.config.js`
    - `backend/src/mus/main.py` (Modify `lifespan` to instantiate and start `PeriodicScanner`, integrate SSE router)
    - `backend/src/mus/application/use_cases/scan_tracks_use_case.py` (Adapt to be called by periodic task, integrate SSE broadcast trigger)
    - `backend/src/mus/domain/entities/player_state.py` (add `is_shuffle`, `is_repeat`)
    - `backend/src/mus/application/dtos/player_state.py` (add `is_shuffle`, `is_repeat`)
    - `backend/src/mus/infrastructure/persistence/sqlite_player_state_repository.py`
    - `backend/src/mus/infrastructure/tasks/background_scanner.py` (New file for PeriodicScanner class)
    - `backend/src/mus/infrastructure/api/sse_handler.py` (New file for SSE logic and endpoint)
    - `backend/tests/api/test_sse_handler.py` (New test file for SSE functionality)
    - `backend/tests/infrastructure/tasks/test_background_scanner.py` (New test file for PeriodicScanner)
    - `backend/tests/test_main_startup.py` (Update for PeriodicScanner task, retained DB/cover cleaning)
    - `backend/tests/application/test_scan_tracks_use_case.py` (Update for SSE broadcast trigger)
  </relevantFiles>
  <everythingElse>
    ## Feature Overview

    *   **Feature Name and ID:** Frontend Redesign & Rebuild v4 (FrR3)
    *   **Purpose Statement:** To comprehensively overhaul the Mus music player's frontend, implementing a new UI/UX. This involves refactoring the SvelteKit application, integrating Shadcn-Svelte, Tailwind CSS, and `xior` for API calls (encapsulated in `apiClient.ts`). Key features include persistent player state (track ID, progress, volume, mute, shuffle, single-track repeat), dynamic tab titles, and dynamic track list updates via Server-Sent Events (SSE) triggered by a backend *periodic background scan (every minute)*. The manual "Scan Library" button and its API endpoint are removed. The backend scanning mechanism will be refactored into a dedicated worker class.
    *   **Problem Being Solved:**
        *   Suboptimal current frontend UI/UX and potential rendering issues.
        *   Lack of player state persistence and robust shuffle/repeat functionalities.
        *   Reliance on manual scans or a single startup scan, and full page reloads for library updates. Transitioning to automatic, periodic background updates.
        *   Backend scanning logic in `main.py` could be better encapsulated and made more testable.
    *   **Success Metrics:**
        *   Frontend UI matches the specification (desktop/mobile), built with Tailwind utilities and Shadcn components minimizing custom CSS.
        *   Player state (including `is_shuffle`, `is_repeat`) saved to backend and correctly restored, with track list scrolling. Default: first track paused if no valid state. `playerStore` uses full `Track` object for UI, but only `track_id` is exchanged with backend.
        *   Dynamic tab title ("Artist - Track Name" if `$playerStore.currentTrack` is set, else "Mus").
        *   Backend performs a *periodic background scan (e.g., every minute)* using a refactored `PeriodicScanner` worker; frontend receives SSE notifications (`tracks_updated` event) and dynamically updates track list via `fetchTracks` without page reload.
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
        *   **Backend:** On startup (`lifespan` event), backend *retains DB reset and cover cleaning operations*. It then instantiates and starts a `PeriodicScanner` background task (e.g., a class in `backend/src/mus/infrastructure/tasks/background_scanner.py`). This task will invoke the `ScanTracksUseCase` to scan the music directory (e.g., every minute) and process tracks asynchronously.
        *   **SSE Notification:** After `ScanTracksUseCase` processes tracks (e.g., per batch or per scan cycle), it triggers the sending of an SSE event (e.g., `data: {\"type\": \"tracks_updated\"}\\n\\n`) to all connected clients via a dedicated SSE endpoint.
        *   **Frontend:** Listens to the SSE endpoint. On receiving the `tracks_updated` event, it calls `apiClient.fetchTracks()` and updates `trackStore` reactively. The manual scan button/endpoint is removed.
    5.  **Shuffle and Repeat (Revised):**
        *   **Shuffle (`isShuffle`):** `nextTrack` picks random from full list (not current). `previousTrack` uses play history; if empty, plays previous in original order.
        *   **Repeat (`is_repeat` for single track):** OFF (Default): Loop entire sequence. ON: Current track loops.
    6.  **Visual Volume Feedback:** Temporary numeric percentage display on volume change.

    #### Technical Requirements (Core as per original spec, plus)

    *   **API Client Encapsulation**: `xior` usage strictly within `frontend/src/lib/services/apiClient.ts`.
    *   **Backend `PlayerState` Update:** Entity, DTO, repository, API to include `is_shuffle` and `is_repeat` (both booleans, defaulting to `False`).
    *   **Backend SSE Endpoint**: New endpoint (e.g., `GET /api/v1/events/track-updates`) for pushing `tracks_updated` notifications.
    *   **Backend Periodic Background Worker**: A dedicated, refactored class (e.g., `PeriodicScanner` in `backend/src/mus/infrastructure/tasks/background_scanner.py`) to manage the periodic scanning logic. This worker is initiated at application startup. It uses a session factory to obtain fresh database sessions for each scan cycle and orchestrates calls to `ScanTracksUseCase`. It's designed for testability and maintainability, reducing direct logic in `main.py`.

    ## Future Considerations (Post-Epic)
    *   Keyboard Shortcuts.
    *   Continuous runtime file system watching (e.g. `watchdog`) for live library updates.
    *   Simple queue/playlist functionality.

  </everythingElse>
</Climb>
