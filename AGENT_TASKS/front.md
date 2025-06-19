# Update Frontend Dependencies and Fix UI Components

## User Problem
The project's frontend dependencies are outdated. This creates a maintenance burden and prevents the use of new features and security patches. Updating these dependencies, particularly `bits-ui` and its related libraries, is expected to cause breaking changes in existing UI components, especially the slider components used for track progress and volume control.

## High-Level Solution
The solution is to update all frontend dependencies to their latest stable versions using `npm-check-updates`. After the update, any resulting build errors or UI component breakages will be systematically resolved. The primary method for fixing broken `bits-ui` based components will be to remove the existing component files and re-add them using the `shadcn-svelte` CLI. This ensures that the components are updated to be compatible with the new dependency versions. The process will be validated by running the CI pipeline to ensure all tests pass and the application builds successfully.

## Success Metrics
- All frontend dependencies in `frontend/package.json` are updated to their latest versions.
- The application successfully builds, and `make ci` passes without any errors, warnings, or test failures.
- All UI components, including buttons, dialogs, and sliders, are fully functional and visually correct.
- The track progress slider in `TrackItem.svelte` and the volume slider in `PlayerFooter.svelte` work as expected.

## Detailed Description
The core of this task is a dependency upgrade followed by a targeted refactoring of UI components that break as a result. The `bits-ui` library, which underpins the `shadcn-svelte` components, is known for introducing breaking changes in major updates.

The expected workflow is as follows:
1.  Use `npm-check-updates` to modify `frontend/package.json` in-place.
2.  Run `npm install` to get the new package versions (better use `make front-install` and docker command as well)
3.  Run `make ci` and observe the failures. These will likely be TypeScript errors in Svelte components due to changed props or component structure from `bits-ui`.
4.  For each broken component (e.g., slider, button), use the `shadcn-svelte` CLI to add it again. This will overwrite the existing component files with fresh, compatible versions.
    - Example command: `npx shadcn-svelte@latest add slider`
5.  After re-adding a component, check the application for any necessary adjustments to how it's used (e.g., prop changes).
6.  Pay close attention to the `Slider` component, as it is used in `TrackItem.svelte` and `PlayerFooter.svelte` and is a known point of failure during these updates. For example thumb usage.
7.  Repeat this process until `make ci` passes cleanly.

**Constraint**: You are strictly forbidden from creating custom component implementations or workarounds. All fixes must be achieved by using the official `shadcn-svelte` components, re-imported via its CLI.

## Subtasks

### [ ] 1. Update Frontend Dependencies and Fix Component Issues
**Description**: Update all frontend dependencies to their latest versions and resolve any resulting breaking changes, particularly with `bits-ui` components.
**Details**:

Move Sliders to On Mount sections. Do not make non-bits ui solution for SSR. Any similar problems have to be moved to on mount section as well.

4.  Run `make ci` to identify build errors and test failures.
5.  Systematically address the issues. For any broken UI components in `frontend/src/lib/components/ui`, remove the component's directory and re-add it using the `shadcn-svelte` CLI.
    - Example: If the slider is broken, run `rm -rf frontend/src/lib/components/ui/slider` and then `npx shadcn-svelte@latest add slider`.
    - Do this for all broken components, which may include `button`, `dialog`, `sheet`, and `slider`.
6.  After re-adding components, check the implementation in files like `frontend/src/lib/components/domain/PlayerFooter.svelte` and `frontend/src/lib/components/domain/TrackItem.svelte` for any required changes to props or event handling, based on the new component structure.
7.  Consult the official changelogs for `bits-ui` and `shadcn-svelte` if you encounter unexpected issues.
8.  Ensure all tests, including those for the affected components (`TrackItem.svelte.test.ts`, `PlayerFooter.svelte.test.ts`, `slider.svelte.test.ts`), pass.
9.  Run `make ci` repeatedly until it passes without any errors.
**Filepaths to Modify**: `frontend/package.json`, `frontend/package-lock.json`, `frontend/src/lib/components/ui/slider/slider.svelte`, `frontend/src/lib/components/ui/button/button.svelte`, `frontend/src/lib/components/ui/dialog/`, `frontend/src/lib/components/ui/sheet/`, `frontend/src/lib/components/domain/PlayerFooter.svelte`, `frontend/src/lib/components/domain/TrackItem.svelte`, `frontend/src/lib/components/ui/slider/slider.svelte.test.ts`
**Relevant Make Commands (Optional)**: `make front-install, make ci`
