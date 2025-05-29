<Climb>
  <header>
    <id>fRntClntFtch</id>
    <type>refactor</type>
    <description>Refactor frontend API client to use native fetch, remove xior, and manage dependencies.</description>
  </header>
  <newDependencies>None anticipated. The goal is to remove `xior`.</newDependencies>
  <prerequisitChanges>None.</prerequisitChanges>
  <relevantFiles>
    - `frontend/package.json`
    - `frontend/src/lib/services/apiClient.ts`
    - `frontend/src/lib/services/apiClient.test.ts`
    - `Makefile` (and `makefiles/frontend.mk` for context on npm commands)
  </relevantFiles>
  <everythingElse>
    **Overall Goal:**
    Reduce frontend bundle size and reliance on external HTTP client libraries by migrating `xior` usage to the native `fetch` API. Additionally, adjust dependency categorization in `package.json`.

    **Task 1: Dependency Management**
    -   **Objective**: Remove `xior` and re-categorize other production dependencies to `devDependencies`.
    -   **Steps**:
        1.  Uninstall `xior` from `frontend/package.json` using `make front-npm-uninstall xior`.
        2.  For each dependency currently listed in `dependencies` in `frontend/package.json` (namely `clsx`, `date-fns`, `lucide-svelte`, `mode-watcher`, `svelte-sonner`, `tailwind-merge`, `tailwind-variants`):
            a.  Uninstall it from `dependencies` using `make front-npm-uninstall <dependency-name>`.
            b.  Reinstall it into `devDependencies` using `make front-npm-dev-install <dependency-name>`.
    -   **Considerations**: Moving runtime dependencies to `devDependencies` is unconventional. While Vite will still bundle used code, this may confuse tools or developers expecting standard `dependencies` listing for runtime code. This is done per user request.

    **Task 2: API Client Refactoring (`frontend/src/lib/services/apiClient.ts`)**
    -   **Objective**: Replace all `xior` HTTP calls with native `fetch` API calls, maintaining functional equivalence.
    -   **Key Changes**:
        -   Remove `import xior from "xior";` and the `api` instance.
        -   `fetchTracks()`: Use `fetch` to get `/tracks`. Return `Track[]` or `[]` on error (log error).
        -   `fetchPlayerState()`: Use `fetch` to get `/player/state`. Return `PlayerState` or `null` on 404/error (log error).
        -   `triggerTestToasts()`: Use `fetch` to get `/events/test_toast`. Throw error if `!response.ok`.
        -   Maintain existing function signatures, return types, and TypeScript types.
        -   Other functions in the file (`getStreamUrl`, `sendPlayerStateBeacon`, `connectTrackUpdateEvents`) that do not use the `xior` instance should remain unaffected.

    **Task 3: Test Updates (`frontend/src/lib/services/apiClient.test.ts`)**
    -   **Objective**: Adapt tests to the `fetch`-based implementation.
    -   **Key Changes**:
        -   Mock `global.fetch` instead of spying on `apiClient`'s exported functions.
        -   Verify `fetch` is called with correct URLs, methods, and headers (if any).
        -   Test successful response parsing for `fetchTracks` and `fetchPlayerState`.
        -   Test error handling:
            -   `fetchTracks`: Ensure `[]` is returned on fetch error.
            -   `fetchPlayerState`: Ensure `null` is returned on 404 or other fetch errors.
            -   `triggerTestToasts`: Ensure an error is thrown when `fetch` response is not ok.
        -   Add new tests for `triggerTestToasts` as they are currently missing.

    **Task 4: Verification**
    -   **Objective**: Ensure all changes integrate correctly and pass quality checks.
    -   **Action**: Run `make ci` from the project root. Resolve any linting errors or test failures.

    **Architectural Decisions based on User Feedback:**
    -   All production dependencies in `frontend/package.json` (except the removed `xior`) will be moved to `devDependencies`.
    -   The error handling for `triggerTestToasts` will involve throwing an error if `!response.ok` to maintain conciseness and existing error propagation patterns.
  </everythingElse>
</Climb>