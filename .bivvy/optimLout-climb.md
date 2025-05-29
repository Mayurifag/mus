<Climb>
  <header>
    <id>optimLout</id>
    <type>task</type>
    <description>Optimize reactive blocks in frontend/src/routes/(app)/+layout.svelte and subsequently refactor audio logic into a dedicated service.</description>
  </header>
  <newDependencies>None anticipated for the optimization phase. The refactoring phase will create a new AudioService.ts file.</newDependencies>
  <prerequisitChanges>None.</prerequisitChanges>
  <relevantFiles>
    - `frontend/src/routes/(app)/+layout.svelte` (Primary target for all tasks)
    - `frontend/src/lib/stores/playerStore.ts` (Referenced heavily)
    - `frontend/src/lib/stores/trackStore.ts` (Referenced heavily)
    - `frontend/src/lib/services/apiClient.ts` (Used for `getStreamUrl` and `sendPlayerStateBeacon`)
    - `frontend/src/lib/services/AudioService.ts` (Will be created during refactoring)
    - Corresponding test files for `+layout.svelte` and potentially for the new `AudioService.ts`.
  </relevantFiles>
  <everythingElse>
    **Overall Goal:**
    Improve performance and maintainability of `frontend/src/routes/(app)/+layout.svelte` by reducing excessive reactive block triggers and refactoring audio logic.

    **Phase 1: Optimizations (Moves 1-6, with Move 6 - console log removal - deferred)**
    1.  **Audio Source Management Consolidation:**
        - Create `updateAudioSource()` function.
        - Integrate logic for `audio.src`, `audio.load()`, and `shouldAutoPlay` management.
        - Modify reactive trigger for `currentTrack` changes.
    2.  **Volume Control Reactivity Optimization:**
        - Move initial volume setup to `onMount`.
        - Simplify reactive block for volume/mute updates.
    3.  **CurrentTime Sync Debouncing:**
        - Introduce `lastSyncTime` and logic to sync `audio.currentTime` only if `timeDiff > 1` AND `(Date.now() - lastSyncTime) > 100ms`.
    4.  **Player State Saving Throttling:**
        - Introduce `lastSaveTime` and logic to save state at most every 2 seconds when playing.
        - Remove `void` expressions, use direct conditional checks.
    5.  **Redundant Title Update Prevention:**
        - Introduce `lastTitleTrackId` to update `document.title` only when track ID changes.
    6.  **Store Initialization Batching:**
        - Move logic processing the `data` prop (for `trackStore` and `playerStore` initialization) into `onMount`.

    **Phase 2: Refactoring (Moves 7-8)**
    1.  **Create AudioService:**
        - Develop `frontend/src/lib/services/AudioService.ts`.
        - This service will manage the `HTMLAudioElement`, handle its events, and contain methods for controlling playback (source, play, pause, volume, seek).
        - It will interact with `playerStore` and `trackStore`.
    2.  **Integrate AudioService into +layout.svelte:**
        - Instantiate `AudioService` in `+layout.svelte`.
        - Delegate audio-related tasks from `+layout.svelte` to the `AudioService`.
        - Remove direct audio manipulation and event handling from `+layout.svelte` where the service now handles it.

    **Important Notes:**
    - Task 6 (Remove Debug Console Logs from `+layout.svelte`) is **deferred** and will not be part of these moves initially. The console logs will be kept for verification during these changes.
    - All changes must ensure existing functionality is preserved.
    - Adherence to Svelte and TypeScript best practices as defined in `.cursor/rules/` is required.
    - `make ci` must pass after each significant set of changes, implying tests should be updated or added as necessary.
  </everythingElse>
</Climb>