# Task Title: Refine TrackItem and PlayerFooter UI

## User Problem
1.  The play/pause button within individual track items (`TrackItem.svelte`) is redundant, as the entire item is already clickable to play/pause the track.
2.  The hover effect on buttons in the `PlayerFooter.svelte` component currently shows a blue background. A more subtle blue glow effect is preferred for these buttons.

## High-Level Solution
1.  Modify `TrackItem.svelte` to remove the dedicated play/pause button element and its specific JavaScript click handler. The main clickable area of the track item will retain its existing play/pause functionality.
2.  Update the Tailwind CSS classes for the `ghost` button variant, primarily defined in `frontend/src/lib/components/ui/button/index.ts`. This change will replace the `hover:bg-accent` class with a class that produces a subtle blue glow (e.g., `hover:shadow-[0_0_10px_hsl(var(--accent)/0.4)]`), while retaining other hover effects like text color changes. This will affect the icon buttons in `PlayerFooter.svelte`.

## Success Metrics
1.  The dedicated play/pause button is no longer visible or interactive on any `TrackItem` component.
2.  Clicking on a `TrackItem` component still correctly plays or selects the track.
3.  Hovering over the primary control icon buttons (Shuffle, Repeat, SkipBack, Play/Pause, SkipForward, Volume toggle, Menu) in `PlayerFooter.svelte` no longer shows a blue background.
4.  Hovering over these specified buttons in `PlayerFooter.svelte` now displays a subtle blue glow effect.
5.  The `make ci` command passes without any errors, warnings, or linter issues.

## Context
The project is a music player application featuring a SvelteKit frontend and a FastAPI backend. Frontend styling is managed using Tailwind CSS, incorporating `shadcn-svelte` components and utility classes. Reactivity is handled with Svelte 5 runes. The requested modifications are focused on frontend UI enhancements within existing Svelte components.

## Detailed Description
### 1. TrackItem Play/Pause Button Removal
-   **Target File**: `frontend/src/lib/components/domain/TrackItem.svelte`
-   **Action**:
    *   Locate the `<Button>` component that renders the play/pause button. This button typically contains `<Play />` or `<Pause />` icons from `@lucide/svelte`.
    *   Remove this entire `<Button>` element block from the Svelte template.
    *   In the `<script>` section, locate and remove the `handlePlayButtonClick` JavaScript function, as it's specifically tied to the removed button.
    *   Ensure that the main `div` element of the track item (the root element with `data-testid="track-item"`) correctly retains its existing `onclick={playTrack}` and `onkeydown={handleKeyDown}` event handlers to preserve the main play/select functionality.
-   **Testing**:
    *   Associated tests in `frontend/src/lib/components/domain/TrackItem.svelte.test.ts` that specifically target the removed play/pause button must be updated or removed.

### 2. PlayerFooter Button Hover Effect Modification
-   **Primary Target File**: `frontend/src/lib/components/ui/button/index.ts`
-   **Affected Component**: `frontend/src/lib/components/layout/PlayerFooter.svelte` (indirectly, via button variant usage).
-   **Action**:
    *   The buttons to be modified in `PlayerFooter.svelte` are the main control icon buttons: Shuffle, Repeat, SkipBack, Play/Pause, SkipForward, Volume toggle, and Menu. These buttons predominantly use the `ghost` variant.
    *   Open `frontend/src/lib/components/ui/button/index.ts`.
    *   In the `buttonVariants` definition, find the `ghost` variant.
    *   Modify its Tailwind CSS classes for the hover state:
        *   Remove the existing `hover:bg-accent` class.
        *   Add a class for the blue glow effect. The suggested class is `hover:shadow-[0_0_10px_hsl(var(--accent)/0.4)]`. The `hsl(var(--accent)/0.4)` part uses the project's accent color variable (typically blue) with 40% opacity for the shadow. The blur radius (`10px`) can be fine-tuned if necessary to achieve the "little bit" of glow.
        *   Ensure that `hover:text-accent-foreground` is retained or that any icon/text color change on hover is visually harmonious with the new glow effect.
-   **Visual Verification**:
    *   After changes, visually inspect the hover effect on the specified buttons in `PlayerFooter.svelte` in the application to confirm the blue background is gone and the blue glow is present and subtle.

## Subtasks

### [x] 1. Remove Play/Pause Button from TrackItem
**Description**: Eliminate the redundant play/pause button from the `TrackItem.svelte` component and remove its associated click handler.
**Details**:
1.  Open the file `frontend/src/lib/components/domain/TrackItem.svelte`.
2.  Locate the `<Button variant="ghost" size="icon" ...>` element that contains the `<Play />` and `<Pause />` icons. This is the dedicated play/pause button.
3.  Delete this entire `<Button>` element and its contents.
4.  In the `<script lang="ts">` section of the same file, find and delete the `handlePlayButtonClick` function.
5.  Verify that the primary `div` element (with `data-testid="track-item"`) still has its `onclick={playTrack}` and `onkeydown={handleKeyDown}` attributes to ensure the track item itself remains interactive for playing the track.
6.  Open `frontend/src/lib/components/domain/TrackItem.svelte.test.ts`. Review the tests. Any tests that specifically assert the presence of, or interaction with, the removed play/pause button should be removed or modified to reflect the change. For example, tests looking for `aria-label="Play track"` or `aria-label="Pause track"` on this specific button will no longer be valid.
7.  Provide the complete, updated content of `frontend/src/lib/components/domain/TrackItem.svelte`.
8.  Provide the complete, updated content of `frontend/src/lib/components/domain/TrackItem.svelte.test.ts`.
**Filepaths to Modify**: `frontend/src/lib/components/domain/TrackItem.svelte`, `frontend/src/lib/components/domain/TrackItem.svelte.test.ts`
**Relevant Make Commands (Optional)**: `make front-lint, make front-test, make ci`

### [ ] 2. Implement Blue Glow Hover Effect for PlayerFooter Buttons
**Description**: Modify the hover styling for `ghost` variant buttons in `PlayerFooter.svelte` to show a blue glow instead of a blue background.
**Details**:
1.  Open the file `frontend/src/lib/components/ui/button/index.ts`.
2.  Find the `buttonVariants` object, and within it, locate the definition for the `ghost` variant. It currently is: `ghost: "hover:bg-accent hover:text-accent-foreground",`.
3.  Modify this line to: `ghost: "hover:shadow-[0_0_10px_hsl(var(--accent)/0.4)] hover:text-accent-foreground",`. This removes the background color on hover and adds a shadow effect using the accent color.
4.  The `hsl(var(--accent)/0.4)` part creates a shadow with the project's accent color (blue) at 40% opacity. The `10px` determines the blur radius of the glow.
5.  Visually test the application by hovering over the icon buttons (Shuffle, Repeat, SkipBack, Play/Pause, SkipForward, Volume toggle, Menu) in the `PlayerFooter.svelte` component to ensure the new glow effect is applied correctly and looks subtle.
6.  Provide the complete, updated content of `frontend/src/lib/components/ui/button/index.ts`.
**Filepaths to Modify**: `frontend/src/lib/components/ui/button/index.ts`
**Relevant Make Commands (Optional)**: `make front-lint, make front-test, make ci`
