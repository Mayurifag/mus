<Climb>
  <header>
    <id>FxP1</id>
    <type>bug</type>
    <description>Address multiple UI/UX bugs, implement player enhancements, refactor frontend components for improved code quality and maintainability, and ensure frontend tests pass.</description>
  </header>
  <newDependencies>None anticipated.</newDependencies>
  <prerequisitChanges>Existing frontend SvelteKit structure and backend API are assumed to be functional as per FrR3 epic completion. No direct backend changes are part of this climb, but successful interaction with existing backend endpoints is required.</prerequisitChanges>
  <relevantFiles>
    - `frontend/src/lib/components/layout/PlayerFooter.svelte`
    - `frontend/src/lib/components/layout/PlayerFooter.svelte.test.ts`
    - `frontend/src/lib/components/domain/TrackItem.svelte`
    - `frontend/src/lib/components/domain/TrackItem.svelte.test.ts`
    - `frontend/src/**/*.test.ts` (and `.spec.ts`, `.svelte.test.ts`, `.svelte.spec.ts`)
    - `frontend/vite.config.ts`
    - `frontend/src/lib/stores/trackStore.ts`
    - `frontend/src/lib/stores/trackStore.test.ts`
    - `frontend/src/routes/(app)/+layout.svelte`
    - `frontend/src/routes/(app)/__tests__/layout.test.ts`
    - `frontend/src/lib/components/ui/slider/slider.svelte` (and its index.ts)
    - `frontend/src/app.css`
    - `frontend/src/routes/(app)/+page.svelte`
    - `frontend/src/routes/(app)/__tests__/page.svelte.test.ts`
    - `frontend/src/lib/components/layout/RightSidebar.svelte`
    - `frontend/src/lib/components/layout/RightSidebar.test.ts`
    - `frontend/src/lib/components/domain/TrackList.svelte`
    - `Makefile` (for `make ci` and frontend test commands)
  </relevantFiles>
  <everythingElse>
    ## Problem Statement and Scope

    This climb addresses a collection of frontend issues, feature enhancements, and code quality improvements identified in the Mus music player. The primary goals are to enhance user experience, fix bugs, and improve the maintainability of the SvelteKit frontend.

    ### Specific Items to Address:

    1.  **Codebase Improvements (Discussion Items Implemented):**
        *   Refactor `PlayerFooter.svelte` to simplify progress and volume slider logic, reducing reliance on multiple `$effect` blocks and promoting direct store interaction.
        *   Refactor `TrackItem.svelte` progress display logic to be more directly derived from global player state if possible, or confirm optimality of current isolated approach.
    2.  **Frontend Test Remediation:** Fix or delete all skipped or failing frontend tests to ensure `make ci` (specifically `make front-test` and `make front-test-coverage`) passes cleanly.
    3.  **Player Bug - Next Track Not Playing:** Resolve issue where the next track is correctly selected upon completion of the current track (or manual skip) but fails to auto-play.
    4.  **Player Bug - Footer Progress Slider Interaction:** Ensure the progress slider in the player footer correctly updates the track's current playback time when dragged by the user.
    5.  **Feature Request - Track Item Progress Slider:** Implement an interactive progress slider directly on the `TrackItem` component (within the tracklist) for the currently playing track, allowing seeking.
    6.  **UX Improvement - Volume Feedback Latency:** Reduce the display duration of the volume percentage feedback in the player footer from 1500ms to 150ms.
    7.  **Styling Bug - Multiple Scrollbars:** Identify and eliminate redundant scrollbars on the page, implementing a single, styled main scrollbar for content overflow (e.g., the track list).
    8.  **UI Cleanup - Remove Specified Text Elements:**
        *   Remove "Music Library" heading from the main page.
        *   Remove "Found X tracks in your library." / "No tracks found..." paragraph from the main page.
        *   Remove "Library Actions" placeholder text from the `RightSidebar` component.
    9.  **UI Layout - Reposition Player Buttons:** Move the "Repeat" and "Shuffle" buttons in the player footer to be positioned to the left of the "Previous Track" button.

    ### Success Metrics:

    *   All listed bugs (items 3, 4, 7) are resolved and verified.
    *   The new feature (item 5) is implemented and functional as described.
    *   UI/UX improvements (items 6, 8, 9) are correctly implemented.
    *   Codebase improvements (item 1) are implemented, leading to cleaner and more maintainable Svelte components.
    *   All frontend unit tests pass, and there are no skipped tests reported by `make front-test`.
    *   `make ci` command passes successfully, indicating no linting errors, formatting issues, or test failures in the frontend.
    *   Manual UI/UX testing confirms the application behaves as expected regarding these changes.
    *   All code changes adhere to the `.augment-guidelines` provided.
  </everythingElse>
</Climb>