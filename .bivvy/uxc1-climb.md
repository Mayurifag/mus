<Climb>
  <header>
    <id>uxc1</id>
    <type>feature</type>
    <description>Revamp frontend color scheme for a dark theme, fix shuffle/repeat button active styling, and implement minor UI/UX enhancements.</description>
  </header>
  <newDependencies>None anticipated.</newDependencies>
  <prerequisitChanges>None.</prerequisitChanges>
  <relevantFiles>
    - `frontend/src/app.css` (for global color variables)
    - `frontend/src/lib/components/domain/TrackItem.svelte` (for selected track background and slider thumb visibility)
    - `frontend/src/lib/components/layout/PlayerFooter.svelte` (for shuffle/repeat button styling)
    - `frontend/tailwind.config.js` (defines how CSS variables are used by Tailwind)
    - `frontend/src/lib/components/domain/TrackItem.svelte.test.ts` (tests for TrackItem)
    - `frontend/src/lib/components/layout/PlayerFooter.svelte.test.ts` (tests for PlayerFooter)
    - `ROADMAP.md` (to update task status)
  </relevantFiles>
  <everythingElse>
    **Feature Overview:**
    The primary goal is to enhance the visual appeal and usability of the frontend by refining the color palette within the existing dark theme, ensuring interactive elements like shuffle/repeat buttons provide clear visual feedback, and improving the visibility of UI components like the progress slider thumb.

    **Functional Requirements:**
    1.  **Color Palette Update:**
        - Main background (tracklist area) should be slightly lighter than pure black but remain dark. (Target: `hsl(0, 0%, 12%)`)
        - Selected track item background should be darker than the main background to ensure the progress bar (using accent color) is clearly visible. (Target: `hsl(0, 0%, 8%)`)
        - Muted elements (hovers, inactive slider tracks, borders) should have their lightness adjusted for better coherence with the new background. (Target for `--muted`: `hsl(0, 0%, 20%)`)
        - Card and Popover backgrounds (e.g., Player Footer) should be slightly darker than the new main background. (Target: `hsl(0, 0%, 10%)`)
        - The overall theme must remain dark, and color usage should be minimized.
    2.  **Shuffle/Repeat Button Styling:**
        - Active shuffle/repeat buttons must clearly indicate their state.
        - Icons for shuffle/repeat must be visible and correctly colored when active (using `hsl(var(--accent))`).
        - A subtle background highlight should be applied to the buttons when active.
    3.  **UI/UX Enhancements:**
        - The progress slider thumb on the currently selected `TrackItem` should be always visible, not just on hover.

    **Technical Requirements:**
    - Color changes should be primarily managed through CSS custom properties in `frontend/src/app.css`.
    - Component-specific styling will be done in their respective Svelte files.
    - Tailwind CSS utility classes should be leveraged.
    - `lucide-svelte` icons are used; ensure they render correctly.
    - All changes must adhere to `.augment-guidelines`.

    **Design and Implementation:**
    - **Colors:**
        - `--background`: Change from `0 0% 3.9%` to `0 0% 12%`.
        - `--muted`: Change from `0 0% 14.9%` to `0 0% 20%`.
        - `--secondary`: Repurpose for selected track background. Change from `0 0% 14.9%` to `0 0% 8%`.
        - `--card` / `--popover`: Change from `0 0% 3.9%` to `0 0% 10%`.
    - **TrackItem:**
        - Selected state: Use `bg-secondary` class.
        - Slider thumb: Modify CSS for constant visibility on selected item.
    - **PlayerFooter:**
        - Shuffle/Repeat icons: Correct `color` attribute to `hsl(var(--accent))` when active.
        - Active button background: Add conditional class like `bg-accent/10`.

    **Testing Approach:**
    - Visual verification of color changes across the application.
    - Functional testing of shuffle/repeat button states.
    - Verification of slider thumb visibility on selected `TrackItem`.
    - Update Vitest unit tests for `TrackItem.svelte` and `PlayerFooter.svelte` to cover changes in conditional classes and styles related to active states or visibility.
  </everythingElse>
</Climb>