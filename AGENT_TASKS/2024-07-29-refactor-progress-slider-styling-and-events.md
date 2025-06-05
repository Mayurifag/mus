# Task: Refactor Progress Slider Styling, Event Handling, and Remove Buffered Feature
[x] Task: Unify progress slider styling, refactor TrackItem event handling, and remove buffered progress bar functionality.

## User Problem
The progress bar styling is inconsistent between the player footer and the track item. The track item's event handling logic for its progress bar and playback controls may be overly complex or contain issues. The current implementation of a buffered progress bar segment needs to be removed to prepare for a different approach in the future.

## High-Level Solution
1.  Modify `Slider.svelte` to remove the `bufferedValue` prop and its associated visual representation.
2.  Update `PlayerFooter.svelte` and `TrackItem.svelte` to remove the `bufferedValue` prop from their `<Slider>` instances.
3.  Verify that the progress sliders in `PlayerFooter.svelte` and `TrackItem.svelte` (when visible) are visually identical, with styling solely controlled by `Slider.svelte`.
4.  Review and refactor event handling logic in `TrackItem.svelte` related to its progress slider interactions and playback controls to improve clarity, correctness, and conciseness. Ensure `TrackItem.svelte` does not apply CSS that overrides `Slider.svelte`'s internal visual characteristics.

## Success Metrics
1.  The `bufferedValue` prop and its visual display (the `bg-accent/30` span) are completely removed from `Slider.svelte`.
2.  `PlayerFooter.svelte` and `TrackItem.svelte` no longer pass or use a `bufferedValue` prop for their `<Slider>` instances.
3.  The progress slider in `PlayerFooter.svelte` and the progress slider in `TrackItem.svelte` (when visible for the selected track) are visually identical.
4.  `TrackItem.svelte` does not contain CSS styling that overrides the internal visual appearance (track color, thumb style, played portion color) of the `Slider.svelte` component. Layout-related styling passed to the `Slider` component (e.g., margins, width) is acceptable.
5.  Event handling logic in `TrackItem.svelte` for slider interaction and playback control is reviewed and refactored, resulting in cleaner, more maintainable, and verifiably correct code.
6.  All relevant Svelte/TypeScript development guidelines from `.augment-guidelines` (e.g., Svelte 5 runes, no `$:`, Tailwind CSS usage) are followed.
7.  `make ci` passes without any errors.

## Context
-   The primary UI components involved are `PlayerFooter.svelte`, `TrackItem.svelte`, and the shared `Slider.svelte` component (`frontend/src/lib/components/ui/slider/slider.svelte`).
-   The `Slider.svelte` component is the source of truth for progress bar styling.
-   The goal is visual consistency, code quality improvement in `TrackItem.svelte`, and removal of the `bufferedValue` feature.
-   Styling is primarily done using Tailwind CSS as per `.augment-guidelines`.
-   Svelte 5 runes are mandatory for reactivity.

## Detailed Description
This task involves modifications across three Svelte components: `Slider.svelte`, `TrackItem.svelte`, and `PlayerFooter.svelte`.

1.  **`Slider.svelte` (Path: `frontend/src/lib/components/ui/slider/slider.svelte`)**:
    *   Remove the `bufferedValue` prop from its `Props` type definition (within the `$props()` destructuring).
    *   Remove the `<span>` element responsible for rendering the buffered part of the progress bar. This is the span:
        ```html
        <span
          class="bg-accent/30 absolute h-full rounded-full"
          style="width: {bufferedValue && bufferedValue.length > 0
            ? (bufferedValue[0] / (max || 100)) * 100
            : 0}%"
        ></span>
        ```
    *   Ensure no other logic or references related to `bufferedValue` remain in this component.

2.  **`TrackItem.svelte` (Path: `frontend/src/lib/components/domain/TrackItem.svelte`)**:
    *   **Slider Prop Removal**: Remove the `bufferedValue` prop from the `<Slider>` component instance. It's currently `<Slider ... bufferedValue={[0]} ... />`.
    *   **Styling Review**:
        *   Examine any local `<style>` block or Tailwind classes applied directly to or passed to the `Slider` component from `TrackItem.svelte`.
        *   Ensure these styles do not alter the *internal* appearance of the slider's track, thumb, or played portion (e.g., colors, heights, border of thumb).
        *   Classes for layout purposes (e.g., `mt-1`, `w-full`) are acceptable. The `Slider.svelte` component should solely define its own visual style.
    *   **Event Handling Refactor**:
        *   Review the following functions for clarity, correctness, and conciseness: `playTrack`, `handlePlayButtonClick`, `handleKeyDown`.
        *   Review slider interaction handlers: `handleProgressChange`, `handleProgressCommit`, `handleInput`, `handleSliderContainerClick` (which calls `event.stopPropagation()`).
        *   Specific attention should be paid to:
            *   Simplifying logic where possible.
            *   Ensuring correct event propagation, particularly `stopPropagation` on slider interactions to prevent the `TrackItem` itself from being selected/played when the slider is directly manipulated.
            *   Adherence to Svelte 5 best practices (runes, avoiding `$:`, clear state management).
            *   Removing any redundant code or unnecessary state variables.
            *   **Preserve** the `console.log` statements that are explicitly marked with "For AI assistant: Debug logging - do not remove this log, user intended to have it".

3.  **`PlayerFooter.svelte` (Path: `frontend/src/lib/components/layout/PlayerFooter.svelte`)**:
    *   **Slider Prop Removal**: Remove the `bufferedValue` prop from the `<Slider>` component instance used for the main playback progress. It's currently `<Slider ... bufferedValue={[0]} ... />`.
    *   **Visual Verification**: After changes, visually confirm that the progress slider in the footer appears identical to the one in a selected `TrackItem`.

4.  **Testing (`frontend/src/lib/components/domain/TrackItem.svelte.test.ts`)**:
    *   Update existing unit tests for `TrackItem.svelte` to reflect:
        *   Removal of the `bufferedValue` prop from the `<Slider>` component.
        *   Any changes to event handling logic.
    *   Ensure all tests pass and provide adequate coverage for the refactored event handlers.
    *   Focus on verifying the interaction logic (clicks, key presses, slider drags) behaves as expected after refactoring.

## Requirements
- Modify the specified Svelte components (`Slider.svelte`, `TrackItem.svelte`, `PlayerFooter.svelte`) and the test file (`TrackItem.svelte.test.ts`).
- Follow Svelte 5 runes and Tailwind CSS conventions as per `.augment-guidelines`.
- Preserve specified `console.log` statements in `TrackItem.svelte`.
- Ensure `make ci` passes without errors.
- All proposed code changes should be complete for the task described.

## Implementation Considerations (Optional)
- When refactoring event handlers in `TrackItem.svelte`, consider if any `$effect` usage can be simplified or if local state management can be made more direct.
- Double-check that removing the `bufferedValue` logic does not inadvertently affect other slider functionalities.

## Filepaths to Modify

frontend/src/lib/components/ui/slider/slider.svelte,
frontend/src/lib/components/domain/TrackItem.svelte,
frontend/src/lib/components/layout/PlayerFooter.svelte,
frontend/src/lib/components/domain/TrackItem.svelte.test.ts

Relevant make commands: make front-dev, make front-lint, make front-test, make ci
