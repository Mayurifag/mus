<Climb>
  <header>
    <id>J7kP</id>
    <type>bug</type>
    <description>Fix music player state restoration issues: ensure correct scroll positioning of the current track to the viewport center without animation, and restore the exact playback position (progress_seconds) with the player paused.</description>
  </header>
  <newDependencies>None.</newDependencies>
  <prerequisitChanges>None.</prerequisitChanges>
  <relevantFiles>
    - `frontend/src/lib/stores/playerStore.ts`
    - `frontend/src/lib/stores/trackStore.ts`
    - `frontend/src/routes/(app)/+layout.svelte`
    - `frontend/src/lib/components/domain/TrackList.svelte`
    - `frontend/src/lib/stores/playerStore.test.ts`
    - `frontend/src/lib/stores/trackStore.test.ts`
    - `frontend/src/routes/(app)/__tests__/layout.test.ts`
    - `frontend/src/lib/components/domain/TrackList.svelte.test.ts`
  </relevantFiles>
  <everythingElse>
    **Objective:** Resolve two state restoration problems in the music player upon initial load.
    1.  **Scroll Positioning:** The currently playing track should be instantly scrolled to the center of the viewport.
    2.  **Playback Position:** The exact playback time (`progress_seconds`) of the current track should be restored, and the player should be paused at this position.

    **Move 1: Implement Playback Position Restoration**

    *   **Modify `frontend/src/lib/stores/playerStore.ts`:**
        *   Change the `setTrack` method signature from `(track: Track, preserveTime = false)` to `(track: Track, initialTime?: number)`.
        *   Update the logic within `setTrack` for `currentTime`:
            ```typescript
            // ...
            currentTime: initialTime ?? 0, // Use initialTime if provided, otherwise default to 0
            // ...
            ```
    *   **Modify `frontend/src/lib/stores/trackStore.ts`:**
        *   Change the `setCurrentTrackIndex` method signature from `(index: number | null)` to `(index: number | null, initialTime?: number)`.
        *   When calling `playerStore.setTrack` within `setCurrentTrackIndex`, pass the `initialTime` parameter:
            ```typescript
            // ...
            if (index !== null && state.tracks[index]) {
              playerStore.setTrack(state.tracks[index], initialTime); // Pass initialTime here
              return { ...state, currentTrackIndex: index };
            }
            // ...
            ```
    *   **Modify `frontend/src/routes/(app)/+layout.svelte`:**
        *   In the `onMount` function, locate the section where `playerState` is restored.
        *   When calling `trackStore.setCurrentTrackIndex`, pass `progress_seconds` from the loaded `data.playerState` as the `initialTime` argument:
            ```typescript
            // ...
            if (trackIndex >= 0) {
              trackStore.setCurrentTrackIndex(trackIndex, progress_seconds); // Pass progress_seconds here
              playerStore.pause();
              trackSelected = true;
            }
            // ...
            ```
        *   The subsequent `playerStore.setCurrentTime(progress_seconds)` call might become redundant if `initialTime` in `setCurrentTrackIndex` handles it. Review if it can be removed or if it serves a purpose for cases where `trackStore.setCurrentTrackIndex` is not called with `initialTime`. For this specific restoration path, ensure `currentTime` is set once correctly. The proposed change to `trackStore` and `playerStore` should make `playerStore.setCurrentTime(progress_seconds)` in `onMount` redundant *for this specific path of restoring a saved track ID*. Confirm and simplify if applicable. (Current analysis suggests the existing `playerStore.setCurrentTime(progress_seconds)` after `trackStore.setCurrentTrackIndex` will correctly override the default 0 set by `playerStore.setTrack` if `initialTime` is not passed to `setTrack`. The proposed change centralizes this, which is cleaner.)
        *   Ensure the `playerStore.pause();` call remains to satisfy the requirement that the player is paused at the restored position.
    *   **Update Tests:**
        *   In `frontend/src/lib/stores/playerStore.test.ts`, add tests for the new `initialTime` parameter in `setTrack`.
        *   In `frontend/src/lib/stores/trackStore.test.ts`, update tests for `setCurrentTrackIndex` to cover passing `initialTime`.
        *   In `frontend/src/routes/(app)/__tests__/layout.test.ts`, ensure tests verify that `currentTime` is correctly restored from `playerState.progress_seconds` and that `playerStore.isPlaying` is `false` after initial load and state restoration.

    **Move 2: Implement Scroll Positioning Fix**

    *   **Modify `frontend/src/lib/components/domain/TrackList.svelte`:**
        *   Import `tick` from `svelte`.
        *   In the reactive block that handles scrolling (`$: if (browser && currentTrackIndex !== null && tracks.length > 0)`):
            *   Remove the `setTimeout` wrapper.
            *   Precede the `document.getElementById` and `scrollIntoView` calls with `await tick();` to ensure the DOM is updated.
            *   Modify the `scrollIntoView` call:
                ```typescript
                // ...
                if (currentTrack) {
                  await tick(); // Ensure DOM is updated
                  const trackElement = document.getElementById(
                    `track-item-${currentTrack.id}`,
                  );
                  if (trackElement) {
                    trackElement.scrollIntoView({
                      behavior: "auto", // For instant scroll
                      block: "center",   // To center the element
                    });
                  }
                }
                // ...
                ```
    *   **Update Tests:**
        *   In `frontend/src/lib/components/domain/TrackList.svelte.test.ts`, update tests to:
            *   Verify that `scrollIntoView` is called with `{ behavior: 'auto', block: 'center' }`.
            *   Mock and verify that `tick` is called before `document.getElementById`.

    **Move 3: Run CI Checks**
    *   Execute `make ci` from the project root.
    *   Ensure all linters, formatters, and tests pass without errors or warnings.

    Adhere to `.cursor/rules/svelte.mdc` and `.cursor/rules/typescript.mdc` for Svelte and TypeScript changes respectively.
    Follow general guidelines in `.augment-guidelines`.
  </everythingElse>
</Climb>
