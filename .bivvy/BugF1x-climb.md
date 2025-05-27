<Climb>
  <header>
    <id>BugF1x</id>
    <type>bug</type>
    <description>Address multiple backend and frontend issues including scanner anomalies, cover processing errors, broken player state persistence, and suboptimal image loading. Includes removal of unused backend placeholder code.</description>
  </header>
  <newDependencies>None anticipated.</newDependencies>
  <prerequisitChanges>
    - Project adheres to `.augment-guidelines`.
    - Existing backend and frontend infrastructure is largely in place as per previous epics (FrR3, FxP1).
  </prerequisitChanges>
  <relevantFiles>
    - `backend/src/mus/application/use_cases/scan_tracks_use_case.py`
    - `backend/src/mus/infrastructure/scanner/file_system_scanner.py`
    - `backend/src/mus/infrastructure/scanner/cover_processor.py`
    - `backend/src/mus/infrastructure/persistence/sqlite_track_repository.py`
    - `backend/src/mus/infrastructure/persistence/sqlite_player_state_repository.py`
    - `backend/src/mus/infrastructure/api/routers/player_router.py`
    - `backend/src/mus/infrastructure/api/routers/track_router.py`
    - `backend/src/mus/application/dtos/player_state.py`
    - `backend/src/mus/main.py`
    - `backend/src/mus/infrastructure/tasks/background_scanner.py`
    - `backend/tests/application/test_scan_tracks_use_case.py`
    - `backend/tests/infrastructure/test_cover_processor.py`
    - `backend/tests/api/test_player_state_api.py`
    - `backend/tests/api/test_track_api.py`
    - `frontend/src/routes/(app)/+layout.svelte`
    - `frontend/src/routes/(app)/+layout.server.ts`
    - `frontend/src/lib/services/apiClient.ts`
    - `frontend/src/lib/stores/playerStore.ts`
    - `frontend/src/lib/stores/trackStore.ts`
    - `frontend/src/lib/components/domain/TrackItem.svelte`
    - `frontend/src/lib/components/domain/TrackList.svelte`
    - `frontend/src/routes/(app)/__tests__/layout.test.ts`
    - `frontend/src/lib/components/domain/TrackList.svelte.test.ts`
    - `backend/src/mus/infrastructure/api/routers/album_router.py` (to be deleted)
    - `backend/src/mus/infrastructure/api/routers/artist_router.py` (to be deleted)
    - `backend/src/mus/infrastructure/api/routers/playlist_router.py` (to be deleted)
    - `backend/src/mus/infrastructure/api/schemas.py` (to be deleted)
    - `backend/src/mus/infrastructure/api/routers/__init__.py` (to be modified)
  </relevantFiles>
  <everythingElse>
    ## Problem Statement and Scope

    This climb addresses a set of critical bugs and improvement areas identified in the Mus music player. The goal is to enhance stability, correctness, and user experience across both backend and frontend components.

    ### Specific Items to Address:

    1.  **Backend - Background Scanner Anomaly:** The background scanner consistently logs "1 added" track on subsequent scans, even when no music files have been modified. This requires debugging the file change detection logic.
    2.  **Backend - VIPS Cover Processing Error:** The `CoverProcessor` frequently fails with `VipsForeignLoad: buffer is not in a known format` when trying to process image data extracted from audio files. This needs to be made more robust.
    3.  **Full-Stack - Player State Persistence:** Saving player state (current track, progress, volume, shuffle/repeat) from the frontend and restoring it upon page load is non-functional. This critical feature must be fixed.
    4.  **Frontend/Backend - Cover Image Loading Optimization:** `GET` requests for cover images (`/api/v1/tracks/.../covers/...`) are described as "noisy." Implement frontend lazy loading for these images and backend HTTP caching headers (ETag, Cache-Control) to improve performance and reduce redundant requests.
    5.  **Frontend - Initial Track Loading UI Blink:** On initial page load, there's a noticeable UI flash where tracks seem to appear, disappear, and then reappear. This needs to be smoothed out for a better user experience.
    6.  **Backend - Code Cleanup:** Remove unused placeholder routers (`album_router.py`, `artist_router.py`, `playlist_router.py`) and the `schemas.py` re-export module to simplify the API structure.

    ### Success Metrics:

    *   **Scanner Anomaly:** Backend logs for the background scanner show 0 added, 0 updated tracks when no actual file changes have occurred between scans.
    *   **VIPS Error:** `VipsForeignLoad` errors related to cover processing are eliminated or significantly reduced, with robust error handling for problematic cover data. Covers are generated correctly for valid embedded images.
    *   **Player State:** Player state (including track ID, progress, volume, mute, shuffle, repeat) is correctly saved to the backend on page unload/hide (via `navigator.sendBeacon` or debounced API calls) and accurately restored when the user revisits the page.
    *   **Cover Loading:** Frontend `TrackItem` components use `loading="lazy"` for cover images. Backend cover endpoints serve appropriate `ETag` and `Cache-Control` headers, and respond with `304 Not Modified` when applicable.
    *   **UI Blink:** The initial loading of the track list on the frontend is smooth, without visual flickering or temporary display of incorrect states.
    *   **Code Cleanup:** Specified placeholder backend routers and the `schemas.py` module are successfully removed without breaking existing functionality.
    *   All changes are accompanied by relevant unit and/or integration tests.
    *   The `make ci` command passes successfully, indicating no linting, formatting, or test failures.
  </everythingElse>
</Climb>