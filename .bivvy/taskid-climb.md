<Climb>
  <header>
    <id>taskid</id>
    <type>feature</type>
    <description>Redesign progress bars in PlayerFooter.svelte and TrackItem.svelte for unified styling, improved interaction, and a new download/buffer progress indicator. Styling changes will be centralized in the shared Slider.svelte component.</description>
  </header>
  <newDependencies>None anticipated.</newDependencies>
  <prerequisitChanges>None.</prerequisitChanges>
  <relevantFiles>
    - `frontend/src/lib/components/layout/PlayerFooter.svelte`
    - `frontend/src/lib/components/domain/TrackItem.svelte`
    - `frontend/src/lib/components/ui/slider/slider.svelte`
    - `frontend/src/lib/components/domain/TrackItem.svelte.test.ts`
    - `frontend/src/lib/components/layout/PlayerFooter.svelte.test.ts`
  </relevantFiles>
  <everythingElse>
    **Unified Design Requirements:**
    1.  Consistent styling between PlayerFooter progress slider and TrackItem progress slider.
    2.  Both progress bars display as horizontal rows.
    3.  Increased height of both progress bars (target: `h-2` or 8px for the track) to improve mouse interaction.
    4.  Same color scheme: accent color for progress (`bg-accent`), muted color for track background (`bg-muted`).

    **Interactive Behavior:**
    1.  Slider thumb hidden by default (`opacity-0`), shown only on hover of the slider component (`group-hover:opacity-100`).
    2.  Clickable for seeking.
    3.  Smooth dragging functionality.

    **Download Progress Indicator:**
    1.  Visual indicator on the progress bar for audio download/buffer progress.
    2.  Secondary progress indicator (e.g., `bg-accent/30` or `bg-accent/50`) showing how much of the audio file has been downloaded/buffered.
    3.  Visually distinct but complementary.
    4.  `Slider.svelte` will accept a `bufferedValue: number[]` prop (placeholder value like `[0]` will be passed from parent components for this task).

    **Technical Implementation:**
    1.  Remove existing CSS for `.track-progress-slider` and `:global(.group\/slider)` from `PlayerFooter.svelte` and `TrackItem.svelte`.
    2.  Implement new styling primarily within `frontend/src/lib/components/ui/slider/slider.svelte` using Tailwind CSS.
    3.  Ensure new styling works with existing Svelte slider components (`bits-ui` based) and their data attributes.
    4.  Maintain accessibility features (aria labels, keyboard navigation).
    5.  Test styling in selected and non-selected `TrackItem` states.
    6.  Minimize new CSS code by centralizing styles in the shared `Slider.svelte` component.

    **Notes:**
    - Integration with `AudioService` for actual buffer progress data is out of scope for this task.
    - The specific height for the progress bar track is `h-2` (8px).
    - Thumb size `h-4 w-4` (16px) will be maintained.
  </everythingElse>
</Climb>