<Climb>
  <header>
    <id>PsF1x</id>
    <type>bug</type>
    <description>Fix player state persistence on page refresh. Ensure current track, progress, volume, and playback modes (shuffle, repeat) are saved via frontend mechanisms (navigator.sendBeacon or debounced POST) and correctly restored, with the track loaded and paused at its last position.</description>
  </header>
  <newDependencies>None anticipated.</newDependencies>
  <prerequisitChanges>
    - Existing frontend SvelteKit structure (`frontend/`) is in place.
    - Existing backend API endpoints for player state (`POST /api/v1/player/state`, `GET /api/v1/player/state`) are assumed to be functional based on user confirmation.
    - Project adheres to `.augment-guidelines`.
  </prerequisitChanges>
  <relevantFiles>
    - `frontend/src/routes/(app)/+layout.svelte` (Primary focus for state saving and restoration logic)
    - `frontend/src/lib/services/apiClient.ts` (For `savePlayerStateAsync` and `navigator.sendBeacon` interaction)
    - `frontend/src/lib/stores/playerStore.ts` (Definition of player state and actions)
    - `frontend/src/lib/stores/trackStore.ts` (Interaction for setting current track)
    - `frontend/src/lib/types/index.ts` (Definition of `PlayerState` DTO)
    - `frontend/src/routes/(app)/+layout.server.ts` (Server-side loading of initial player state)
    - `frontend/src/routes/(app)/__tests__/layout.test.ts` (Unit tests for layout logic)
    - `backend/src/mus/infrastructure/api/routers/player_router.py` (For reference, though backend changes are not expected)
    - `backend/src/mus/infrastructure/persistence/sqlite_player_state_repository.py` (For reference)
  </relevantFiles>
  <everythingElse>
    ## Problem Statement

    Player state (current track ID, progress, volume level, mute status, shuffle mode, repeat mode) is not correctly persisting when the user refreshes the page. This results in the player resetting to a default state instead of restoring the user's last session state. The issue is suspected to be on the frontend, potentially with the `POST` request for state saving not being triggered or correctly formulated.

    ## Scope and Solution

    This climb will focus on debugging and fixing the frontend mechanisms responsible for saving player state, primarily `navigator.sendBeacon` (triggered on `beforeunload`/`visibilitychange`) and potentially debounced `POST` requests made via `apiClient.ts`.

    The solution involves:

    1.  **Refactoring `apiClient.ts`**: Remove the `quickApi` instance. The `savePlayerStateAsync` function will use the main `api` instance for POSTing player state.
    2.  **Frontend Logging and Debugging**:
        *   Add extensive logging in `frontend/src/routes/(app)/+layout.svelte` to trace the creation of the `PlayerStateDTO` and the attempts to transmit it via `navigator.sendBeacon` and `debouncedSavePlayerState`.
        *   Use browser developer tools to inspect network requests (especially beacon requests which are harder to track).
    3.  **State Saving Logic Correction**: Based on the debugging, fix any issues in `+layout.svelte` that prevent the correct DTO from being formed or sent. This includes ensuring all relevant fields (`current_track_id`, `progress_seconds`, `volume_level`, `is_muted`, `is_shuffle`, `is_repeat`) are included and accurate.
    4.  **State Restoration Logic Correction**: Verify and fix the logic in `+layout.svelte` (`onMount`) that restores the player state from `data.playerState`. The player should load the correct track and be paused at the last known `progress_seconds`. All other state aspects (volume, mute, shuffle, repeat) must also be restored.
    5.  **Testing**: Augment frontend unit tests and perform thorough manual testing of page refresh scenarios.

    The backend endpoints for saving and retrieving player state (`/api/v1/player/state`) are assumed to be working correctly as per user indication.

    ## Success Metrics

    *   Player state (current track ID, progress in seconds, volume level, mute status, shuffle mode, repeat mode) is consistently saved when the page is refreshed or hidden.
    *   Upon page load after a refresh, the player state is accurately restored:
        *   The correct track is loaded.
        *   The playback progress is set to the last known position.
        *   The player is paused (even if it was playing before refresh).
        *   Volume, mute, shuffle, and repeat settings are restored.
    *   The frontend uses the main `apiClient` instance for saving player state (no `quickApi`).
    *   Frontend console logs clearly show state saving attempts during debugging (these logs will be removed in the final commit).
    *   Relevant frontend unit tests pass.
    *   `make ci` command completes successfully.
  </everythingElse>
</Climb>