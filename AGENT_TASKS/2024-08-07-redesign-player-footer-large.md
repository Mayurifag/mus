# Task Title
[ ] Task: Redesign PlayerFooter component for larger size and improved layout.

## User Problem
The current player footer is too small, and the layout of controls (especially volume) is suboptimal. Users need a more prominent and user-friendly player footer with better visual hierarchy and easier interaction.

## High-Level Solution
Modify `PlayerFooter.svelte` by increasing its overall height and the size of its internal elements (album art, buttons, progress slider). Relocate the volume control group (mute button and volume slider) to be to the right of the "Next Track" button. The main track progress slider will be positioned underneath the entire cluster of control buttons. All styling will be done using Tailwind CSS, ensuring existing functionality, responsiveness (with primary focus on desktop/tablet for now), and accessibility are maintained.

## Success Metrics
- `PlayerFooter` component height is `h-28` (112px).
- Album art image is `h-18 w-18` (72px).
- Main control buttons are resized: Play/Pause to `h-12 w-12` (icon `h-7 w-7`), Prev/Next to `h-10 w-10` (icons `h-6 w-6`).
- Shuffle/Repeat buttons are `h-9 w-9` (icons `h-5 w-5`). Mute button also `h-9 w-9` (icon `h-5 w-5`).
- Track title font is `font-medium text-base`, artist font is `text-sm`.
- Volume control group is positioned to the right of the "Next Track" button, within the central controls cluster.
- Main track progress slider is positioned below the central control button cluster and spans its width.
- All existing functionalities (play, pause, next, previous, shuffle, repeat, volume control, mute, progress scrubbing, buffered ranges display) are preserved and working correctly.
- The component remains responsive, focusing on desktop/tablet views for this iteration (mobile considerations are deferred by user).
- `make ci` passes successfully.

## Context
The task involves refactoring the `PlayerFooter.svelte` component. Key changes include sizing, typography, and layout adjustments, particularly for the volume controls and progress slider. The component uses Svelte 5 runes and Tailwind CSS for styling. `AudioService` and `trackStore` are integral to its functionality. `TrackItem.svelte` serves as a reference for typography.

## Detailed Description
The redesign aims for a more modern and user-friendly player interface.
**Overall Layout:**
- The footer's height will be `h-28`.
- The three-column structure (Track Info, Controls, Volume/Menu) will be adapted. The central section housing controls will now be more flexible to accommodate the repositioned volume controls and the progress slider beneath them.
**Left Section (Track Info):**
- Album art: `h-18 w-18`.
- Track Title: `font-medium text-base`.
- Artist Name: `text-sm`.
**Center Section (Controls & Progress):**
- **Control Buttons Row:**
    - Shuffle: Button `h-9 w-9`, Icon `h-5 w-5`.
    - Repeat: Button `h-9 w-9`, Icon `h-5 w-5`.
    - Previous Track: Button `h-10 w-10`, Icon `h-6 w-6`.
    - Play/Pause: Button `h-12 w-12`, Icon `h-7 w-7`.
    - Next Track: Button `h-10 w-10`, Icon `h-6 w-6`.
    - Mute: Button `h-9 w-9`, Icon `h-5 w-5`.
    - Volume Slider: Standard width (e.g., `w-32`), positioned next to Mute button.
- **Progress Slider Row (Below Control Buttons):**
    - The main track progress slider will be wider, positioned under the entire cluster of control buttons mentioned above. It should span the width of this central control "block".
    - Time indicators (current time, total duration) will flank the progress slider.
    - Buffered ranges visualization on the progress slider must remain intact.
**Right Section (Mobile Menu):**
- The mobile menu button (`md:hidden`) remains.
**Styling and Functionality:**
- All changes must use Tailwind CSS classes as per `.augment-guidelines`.
- All interactive elements must have appropriate `aria-label` attributes and maintain good touch targets.
- Existing props (`audioService`) and event handling related to player functionality must be preserved.

## Requirements
- Preserve all existing player functionalities.
- Ensure changes are responsive (desktop/tablet primary focus).
- Maintain accessibility standards.
- Component props and event handling should remain unchanged.

## Implementation Considerations
- Adhere to Svelte 5 runes and Tailwind CSS practices as outlined in `.augment-guidelines`.
- The central control section will likely use `flex-col` for the button row and progress row, and the button row itself will be `flex items-center`.
- The existing `w-1/3` class for sections may need to be removed or adjusted (e.g., to `flex-shrink-0 w-auto` for the left/right sections and `flex-1` for the central section) to allow the central area to expand appropriately for the new control cluster and progress slider layout.
- Consider the visual hierarchy and spacing carefully to ensure the larger elements and new layout feel balanced and uncluttered.

## Subtasks

### [ ] 1. Increase Component Size and Resize/Restyle Core Elements
**Description**: Increase overall footer height, resize album art, standardize typography, and resize all control buttons.
**Details**: Modify `PlayerFooter.svelte`.
- Set the main footer `div` height to `h-28`.
- Update the album art `img` (and its container `div`) to `h-18 w-18`.
- Apply `font-medium text-base` Tailwind classes to the track title `span` and `text-sm` to the artist name `span`.
- Resize control buttons and their icons:
    - Play/Pause button: `h-12 w-12`. Icon: `h-7 w-7`.
    - Previous/Next buttons: `h-10 w-10`. Icons: `h-6 w-6`.
    - Shuffle/Repeat buttons: `h-9 w-9`. Icons: `h-5 w-5`.
    - Mute button (in its current position for this subtask): `h-9 w-9`. Icon: `h-5 w-5`.
- The main progress slider and volume slider are *not yet moved or significantly restyled* in this subtask, beyond minor adjustments needed if their container width changes due to button resizing.
- The existing three main sections (left info, center controls, right volume/menu) might need their flex-basis (e.g., `w-1/3`) adjusted to prevent overflow with larger elements, but major re-structuring of the center section (moving volume controls, placing progress slider under buttons) is deferred to the next subtask.
- Ensure all existing functionality is preserved.
**Filepaths to Modify**: ``` `frontend/src/lib/components/layout/PlayerFooter.svelte` ```

### [ ] 2. Relocate Volume & Progress Slider, Finalize Layout, Styling, and Testing
**Description**: Move the volume control group, reposition the main progress slider under the control button cluster, finalize all styling, and update unit tests.
**Details**: In `PlayerFooter.svelte`:
- Move the (already resized from Subtask 1) Mute button and Volume slider to be positioned to the right of the "Next Track" button, forming part of the central control button cluster.
- Restructure the central control section into two rows:
    1.  **Top Row (Control Buttons)**: Horizontally arrange Shuffle, Repeat, Previous, Play/Pause, Next, Mute button, and Volume slider. Ensure proper alignment and spacing.
    2.  **Bottom Row (Progress Slider)**: Position the main track progress slider here. It should be made wider to span the full width of the control button cluster above it. Ensure time indicators
    (current time, total duration) are correctly positioned relative to this slider.
- Both rows have to have same width and aligned
- Verify the `bufferedRanges` prop is correctly passed to the main progress `Slider` and its visualization remains functional.
- Finalize all Tailwind CSS for spacing, alignment, and overall visual harmony across the entire footer to ensure a polished look.
- Confirm responsiveness for desktop and tablet views. Mobile-specific volume control layout adjustments are deferred per user clarification.
- Update unit tests in `PlayerFooter.svelte.test.ts` to reflect all layout and sizing changes. Ensure tests for all interactive elements pass. Add new tests if necessary for the new control arrangement (e.g., progress slider under buttons).
**Filepaths to Modify**: ``` `frontend/src/lib/components/layout/PlayerFooter.svelte`, `frontend/src/lib/components/layout/PlayerFooter.svelte.test.ts` ```
**Relevant Make Commands (Optional)**: ``` `make front-test`, `make front-lint`, `make ci`
