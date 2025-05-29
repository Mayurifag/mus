<Climb>
  <header>
    <id>fR7b</id>
    <type>task</type>
    <description>Refactor frontend components and state management for track loading, player state saving, auto-scroll behavior, and improve backend logging for player state.</description>
  </header>
  <newDependencies>None anticipated.</newDependencies>
  <prerequisiteChanges>None anticipated beyond the existing codebase.</prerequisiteChanges>
  <relevantFiles>
    - `frontend/src/lib/stores/trackStore.ts`
    - `frontend/src/lib/stores/playerStore.ts`
    - `frontend/src/lib/components/domain/TrackList.svelte`
    - `frontend/src/routes/(app)/+layout.svelte`
    - `frontend/src/routes/(app)/+layout.server.ts`
    - `frontend/src/lib/services/apiClient.ts`
    - `backend/src/mus/infrastructure/api/routers/player_router.py`
    - `frontend/src/lib/stores/trackStore.test.ts`
    - `frontend/src/lib/components/domain/TrackList.svelte.test.ts`
    - `frontend/src/routes/(app)/__tests__/layout.test.ts`
    - `frontend/src/lib/services/apiClient.test.ts`
  </relevantFiles>
  <everythingElse>
    <FunctionalRequirements>
      1.  **Eliminate `isLoading` State**:
          - Remove `isLoading` variable and associated logic from `frontend/src/lib/stores/trackStore.ts`.
          - Remove `isLoading` prop and conditional rendering related to it from `frontend/src/lib/components/domain/TrackList.svelte`. The component will rely on `tracks.length === 0` for its empty state.
      2.  **`currentTrackIndex` Nullability Logic**:
          - In `frontend/src/lib/stores/trackStore.ts`, ensure `currentTrackIndex` is `null` if the `tracks` array is empty.
          - If `tracks` is not empty and no valid prior player state exists, `currentTrackIndex` should default to `0`.
          - The `setCurrentTrackIndex` method in the store will be updated to enforce this: if `tracks` is empty, `currentTrackIndex` becomes `null`; if `tracks` is not empty and an invalid index is provided, it defaults to `0`.
      3.  **Redundant Reactive Assignment in `TrackList.svelte`**:
          - The line `$: currentTrackIndex = $trackStore.currentTrackIndex;` in `frontend/src/lib/components/domain/TrackList.svelte` will be removed.
          - All usages of the local `currentTrackIndex` will be updated to use `$trackStore.currentTrackIndex` directly.
      4.  **Single-Execution Auto-Scroll**:
          - The auto-scroll logic will be removed from `frontend/src/lib/components/domain/TrackList.svelte`.
          - New auto-scroll logic will be added to `frontend/src/routes/(app)/+layout.svelte`.
          - This logic will execute only once upon initial page load after `data.tracks` and `data.playerState` are processed and the relevant stores are initialized.
          - It will identify the `currentTrackIndex` from the store. If valid, it will find the DOM element `track-item-{currentTrack.id}` and scroll it into view using `{ behavior: "auto", block: "center" }`.
          - An `initialScrollDone` flag will prevent re-triggering.
      5.  **Refactor Player State Saving and Backend Logging**:
          - A new helper function, `constructPlayerStateDTO()`, will be created in `frontend/src/routes/(app)/+layout.svelte` to build the `PlayerState` DTO from `$playerStore`.
          - The `debouncedSavePlayerState` function in `frontend/src/routes/(app)/+layout.svelte` will use `constructPlayerStateDTO()` and then call `apiClient.savePlayerStateAsync(dto)`.
          - The `sendPlayerStateBeacon` function in `frontend/src/routes/(app)/+layout.svelte` will use `constructPlayerStateDTO()`. The URL for `navigator.sendBeacon` will be constructed using `API_BASE_URL` from `apiClient.ts`.
          - `frontend/src/lib/services/apiClient.ts`'s `savePlayerStateAsync` function will ensure it correctly posts the DTO.
          - The `save_player_state` endpoint in `backend/src/mus/infrastructure/api/routers/player_router.py` will be updated to log the received `PlayerStateDTO` as a JSON string (e.g., `logger.info(f"Received player state: {player_state.model_dump_json(indent=2)}")`).
    </FunctionalRequirements>
    <TechnicalRequirements>
      - Adhere to project's ESLint, Prettier, and other linting/formatting configurations.
      - Follow Svelte, TypeScript, and Python best practices as defined in `.cursor/rules/`.
    </TechnicalRequirements>
    <TestingApproach>
      - Update existing Vitest unit tests for all modified Svelte components and stores (`.test.ts` files).
      - Add new Vitest unit tests where necessary to cover new or significantly refactored logic.
      - Ensure backend Python tests continue to pass after the logging addition.
      - Perform manual verification of all changed functionalities in the browser.
      - Run `make ci` to ensure all automated checks pass.
    </TestingApproach>
  </everythingElse>
</Climb>