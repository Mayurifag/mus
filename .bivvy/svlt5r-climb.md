<Climb>
  <header>
    <id>svlt5r</id>
    <type>task</type>
    <description>Refactor the Svelte frontend codebase to use Svelte 5 runes, replacing reactive declarations (`$: `), refactoring multi-state update functions, adding debug logging to effects, updating tests, and updating Svelte development guidelines.</description>
  </header>
  <newDependencies>None anticipated. Svelte 5 is an upgrade of an existing dependency.</newDependencies>
  <prerequisitChanges>A Svelte 5 compatible development environment and toolchain must be set up.</prerequisitChanges>
  <relevantFiles>
    - `frontend/src/routes/(app)/+layout.svelte`
    - `frontend/src/lib/components/layout/PlayerFooter.svelte`
    - `frontend/src/lib/components/domain/TrackItem.svelte`
    - `frontend/src/lib/components/domain/TrackList.svelte`
    - `frontend/src/lib/stores/trackStore.ts` (for understanding interactions)
    - `frontend/src/lib/services/AudioService.ts` (for understanding interactions)
    - All `.svelte` files in `frontend/src/` for general review.
    - Associated test files (`*.test.ts`) for the Svelte components.
    - `.cursor/rules/svelte.mdc` (guideline file)
  </relevantFiles>
  <everythingElse>
    **1. Rune Migration Details:**
    - All instances of `$: variable = expression;` must be replaced with `variable = $derived(expression);` (if `variable` is state) or `const variable = $derived(expression);` (if it's a read-only computed value).
    - All instances of `$: { statement; }` or `$: if (condition) { statement; }` must be replaced with `$effect(() => { statement; });` or `$effect(() => { if (condition) { statement; } });`.
    - Use `$effect.pre` for effects that need to run before DOM updates if any such cases exist.
    - Existing component state declared with `let` and made reactive by `$: ` should be refactored to use `$state()`.
    - Props should be accessed via `$props()`.
    - Two-way bindings should use `$bindable()`.

    **2. Function Refactoring Details:**
    - This applies primarily to Svelte component functions.
    - If a function `foo()` does `this.stateA = 1; this.stateB = 2;` where `stateA` and `stateB` are distinct `$state()` variables, it should be split into `updateStateA(value)` and `updateStateB(value)`.
    - Store actions (e.g., in `trackStore.ts`) or service methods (e.g., in `AudioService.ts`) that update multiple properties of a single state object as part of a cohesive unit are generally acceptable and not the primary target for this refactoring, unless their internal logic within a component context can be improved by runes.

    **3. Debug Logging Specifics:**
    - Inside every `$effect` and `$effect.pre` block, add a `console.log(...)` statement.
    - Directly above each `console.log`, add the comment: `// For AI assistant: Debug logging - do not remove this log, user intended to have it`.
    - The log message should identify the component (if not obvious from file context) and the purpose/trigger of the effect, along with values of relevant state being read or changed. Example: `console.log("PlayerFooter $effect: currentTrack or isPlaying changed. Playing: ", $audioServiceStore()?.isPlaying, "Track ID: ", $trackStore.currentTrack?.id);`

    **4. Test Update Considerations:**
    - Changes from `$: ` to runes can alter component update timing or how reactivity is triggered. Tests relying on specific update cycles or direct state inspection might need adjustments.
    - Ensure tests for components correctly mock or provide Svelte 5 rune-based state and props if necessary.
    - Test component interactions that trigger effects to ensure the effects run as expected.
    - Do not test the `console.log` statements themselves unless they break existing assertions.

    **5. Guideline Updates (`.cursor/rules/svelte.mdc`):**
    - Emphasize Svelte 5 runes (`$state`, `$derived`, `$effect`, `$effect.pre`) as the standard for reactivity.
    - Explicitly disallow the use of `$: ` syntax (both for derived values and statements) in new code. Existing code will be refactored.
    - Update examples for prop definition using `$props()`.
    - Update examples for two-way binding using `$bindable()`.
    - Revise state management section to reflect rune-first approach for component state.
    - Recommend direct usage of `$derived` and `$effect` over `$: ` sugar.
    - Mention `$props()` and `$bindable()` as Svelte 5 best practices for props and bindings.

    **6. Opportunistic Refactoring (for files already using Svelte 5):**
    - For files like `TrackItem.svelte` and `TrackList.svelte` that already use runes, review their implementation.
    - If there are clearer, more idiomatic, or more performant ways to achieve the same reactivity or logic using Svelte 5 runes (e.g., simplifying effect dependencies, using `$derived` more effectively for intermediate computations), these enhancements should be made.
    - This is not a major architectural rewrite but an opportunity to refine existing rune usage based on best practices.
  </everythingElse>
</Climb>
