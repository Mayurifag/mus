<!-- AGENT_TASKS/refactor-track-item-state.md -->
# Refactor TrackItem Playback State Management

## User Problem
The current method for updating the playback progress slider within each `TrackItem.svelte` component is inefficient. It uses an individual `$effect` in each component to subscribe to global audio state stores, leading to excessive component re-renders, performance degradation during scrolling, and unwanted log messages in the console.

## High-Level Solution
The solution is to centralize the audio playback state management within the parent `TrackList.svelte` component. This component will subscribe to the necessary audio service stores once. It will then pass the reactive playback state values (`currentTime`, `duration`, `isPlaying`, etc.) as props only to the single, currently selected `TrackItem` component. This change converts `TrackItem` into a more performant, "dumb" presentational component that simply renders the props it receives, eliminating the need for it to have its own effects for subscribing to global state.

## Success Metrics
- The `$effect` and its associated `console.log` statements within `TrackItem.svelte` that subscribe to audio state stores are completely removed.
- The playback progress slider for the currently playing track continues to function correctly, reflecting the real-time audio progress.
- The UI remains performant, with no noticeable lag when scrolling through the track list during playback.
- The console is free of the "currentTime updated for track" log spam originating from `TrackItem.svelte`.

## Detailed Description
The refactoring involves two main components: `TrackList.svelte` and `TrackItem.svelte`.

1.  **`TrackList.svelte` Modifications**:
    -   This component will now be responsible for listening to state changes from the `AudioService`.
    -   It will hold local reactive state variables for `isPlaying`, `currentTime`, `duration`, and `bufferedRanges`.
    -   A new `$effect` will be added to subscribe to the corresponding stores in the `audioService` prop. This centralizes all subscriptions into one place.
    -   Inside the virtualized `#each` loop, it will conditionally pass the playback state variables as props to the `TrackItem` component, but only if that item is the currently selected one.

2.  **`TrackItem.svelte` Modifications**:
    -   The component will be simplified by removing its internal `$effect` that subscribes to the `audioService` stores. This also removes the source of the user's logged messages.
    -   It will be updated to accept new optional props: `currentTime?: number`, `duration?: number`, `isPlaying?: boolean`, and `bufferedRanges?: TimeRange[]`.
    -   Internal state variables (`localIsPlaying`, `duration`, `bufferedRanges`) will be removed and replaced by the new props, making it a "dumb" component that just reflects the state it's given.
    -   The `progressValue` state for the slider will be driven by the `currentTime` prop.

3.  **Test Updates**:
    -   The unit tests for `TrackItem.svelte` (`TrackItem.svelte.test.ts`) will be updated to reflect its new props-based API. Instead of mocking the `AudioService` and its stores, tests will now pass props directly to simulate different playback states and verify the component's rendering.

This approach follows best practices for Svelte 5 by co-locating state subscriptions with the component that manages the list (`TrackList`) and passing derived state down to children, rather than having many children subscribe to the same global state independently.

## Subtasks

### [x] 1. Refactor TrackList and TrackItem for Centralized State
**Description**: Modify `TrackList` to manage audio state subscriptions and pass state as props to the selected `TrackItem`, which will be refactored to be a "dumb" component.
**Details**:
- In `frontend/src/lib/components/domain/TrackList.svelte`:
    - Import the `TimeRange` type.
    - Create local state variables for `isPlaying`, `currentTime`, `duration`, and `bufferedRanges`.
    - Add a single `$effect` to subscribe to the corresponding stores from the `audioService` prop (`isPlayingStore`, `currentTimeStore`, `durationStore`, `currentBufferedRangesStore`). Ensure the effect cleans up subscriptions.
    - In the `#each` loop that renders `TrackItem` components, pass the new state variables as props only to the `TrackItem` that is currently selected. For non-selected items, these props will be `undefined`.
- In `frontend/src/lib/components/domain/TrackItem.svelte`:
    - Remove the entire `$effect` block that subscribes to `audioService` stores.
    - Remove all `console.log` statements within the script block.
    - Add new optional props: `currentTime?: number`, `duration?: number`, `isPlaying?: boolean`, `bufferedRanges?: TimeRange[]`.
    - Remove the local state variables `localIsPlaying`, `duration`, and `bufferedRanges`.
    - Update the template to use the new props directly (e.g., use the `isPlaying` prop instead of `localIsPlaying`).
    - The `progressValue` state should remain but be driven by the `currentTime` prop. Ensure it resets to 0 when the item is no longer selected.
- In `frontend/src/lib/components/domain/TrackItem.svelte.test.ts`:
    - Update the tests to align with the new props-based API. Instead of mocking `AudioService` stores to test the component, pass props directly to simulate the selected state (e.g., `render(TrackItem, { isSelected: true, currentTime: 30, ... })`).
**Filepaths to Modify**: `frontend/src/lib/components/domain/TrackList.svelte,frontend/src/lib/components/domain/TrackItem.svelte,frontend/src/lib/components/domain/TrackItem.svelte.test.ts`
