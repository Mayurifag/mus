<Climb>
  <header>
    <id>aDecNtTtl</id>
    <type>refactor</type>
    <description>Decouple AudioService.handleEnded from TrackStore.nextTrack and move document.title setting into AudioService.</description>
  </header>
  <newDependencies>None anticipated.</newDependencies>
  <prerequisitChanges>
    The `optimLout` climb, which introduced `AudioService.ts` and refactored audio logic into it, must be complete. The current state of `AudioService.ts` (managing HTMLAudioElement, events, and basic playback controls) is the baseline.
  </prerequisitChanges>
  <relevantFiles>
    - `frontend/src/lib/services/AudioService.ts`
    - `frontend/src/lib/services/AudioService.test.ts`
    - `frontend/src/routes/(app)/+layout.svelte`
    - `frontend/src/routes/(app)/__tests__/layout.test.ts`
    - `frontend/src/lib/stores/trackStore.ts` (for context on `nextTrack`)
  </relevantFiles>
  <everythingElse>
    **Overall Goal:**
    Enhance separation of concerns and streamline responsibilities within the audio playback architecture.

    **Primary Task 1: Decouple `AudioService.handleEnded` from `TrackStore.nextTrack`**
    -   **Problem**: `AudioService.handleEnded` directly calls `this.trackStore.nextTrack()`.
    -   **Solution**: Modify `AudioService` to accept an `onPlaybackFinishedWithoutRepeat: () => void` callback in its constructor. This callback will be invoked in `handleEnded` if the track is not repeating. `+layout.svelte` will provide `() => trackStore.nextTrack()` as this callback during `AudioService` instantiation.
    -   **Impact**: `AudioService` will no longer have a direct dependency on `TrackStore` for advancing to the next track.

    **Primary Task 2: Move `document.title` Setting into `AudioService`**
    -   **Problem**: `document.title` is currently set via a reactive Svelte statement in `+layout.svelte` based on `$trackStore.currentTrack`.
    -   **Solution**: The `AudioService.updateAudioSource()` method will now be responsible for setting `document.title = \`${track.artist} - ${track.title}\`;` when a new track starts playing. It will include logic to prevent redundant updates if the track ID hasn't changed. The corresponding reactive logic and state variables will be removed from `+layout.svelte`.
    -   **Impact**: `AudioService` centralizes another piece of audio-state-dependent UI update. `+layout.svelte` is simplified. The document title will persist with the last playing track's information if no track is subsequently played; no default title resetting logic will be added to `AudioService` for when a track becomes null.

    **Key Architectural Decisions based on User Feedback:**
    -   The callback approach is preferred over introducing a new Svelte store for signaling track completion from `AudioService`.
    -   `AudioService` will set the `document.title` based on the active track. It will *not* reset the title to a default if the current track becomes null; the title of the last playing track will persist.

    **Testing Considerations:**
    -   Unit tests for `AudioService.ts` will verify the new constructor, the correct invocation of the `onPlaybackFinishedWithoutRepeat` callback, and the `document.title` setting logic (including mocking `document`).
    -   Tests for `+layout.svelte` will be updated to reflect the new way `AudioService` is instantiated and how `nextTrack` is triggered upon track completion.
  </everythingElse>
</Climb>