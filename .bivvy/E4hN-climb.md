---
id: E4hN
type: feature
description: Enhance backend with configurable music directory, conditional CORS, development startup routines (DB reset, async scan), simplified track listing, frontend dependency refactor (including Autoprefixer and PostCSS removal), and temporarily remove E2E tests with ARCHITECTURE.md update.
---

## Feature Overview

*   **Feature Name and ID:** Backend Enhancements, Frontend Dependency Refactor & PostCSS Removal, and E2E Test Restructure Prep (E4hN)
*   **Purpose Statement:** To improve the development experience, configurability, and API structure of the Mus backend; refactor frontend dependencies for clarity and correctness (including removing Autoprefixer and PostCSS); and to temporarily remove the current E2E testing setup while updating `ARCHITECTURE.md` to reflect a future E2E strategy. This includes making the music source directory configurable, adjusting CORS behavior for dev/prod, automating data reset/scan on dev startup, simplifying track listing, correcting `package.json` dependency categorizations, and removing explicit PostCSS setup.
*   **Problem Being Solved:**
    *   Incorrect categorization of some frontend runtime dependencies as `devDependencies`.
    *   Presence of unused or implicitly handled frontend dependencies (`@playwright/test`, `autoprefixer`, `postcss`).
    *   Current E2E testing setup needs revision; temporary removal allows focused development and planning for a new strategy.
    *   Hardcoded music directory limits development flexibility.
    *   Static CORS policy not ideal for differing dev/prod needs.
    *   Manual data reset and scanning during development is cumbersome.
    *   Paginated track listing API is currently an over-optimization for the frontend's needs.
    *   Explicit PostCSS setup might be redundant if handled by `@tailwindcss/vite`.
*   **Success Metrics:**
    *   Frontend `package.json` correctly categorizes runtime and development dependencies.
    *   `@playwright/test`, `autoprefixer`, and `postcss` are removed from frontend dependencies.
    *   `frontend/postcss.config.js` is removed.
    *   Documentation (`ARCHITECTURE.md`, `README.md`) updated to reflect PostCSS removal.
    *   A `front-npm-uninstall` make target is available and used.
    *   All current E2E testing infrastructure (files, configs, scripts from `frontend/e2e/`) is removed.
    *   `ARCHITECTURE.md` is updated to remove current E2E details and outline a future E2E testing section.
    *   `GET /api/v1/tracks` returns a simple list of all tracks. `PagedResponseDTO` is removed.
    *   CORS middleware is applied conditionally based on `APP_ENV`.
    *   On startup the database is reset, covers folder is cleaned, and music scanning starts asynchronously.
    *   The music directory for scanning is configurable via the `MUSIC_DIR` environment variable, defaulting to `./music`.
    *   All changes are covered by appropriate unit/integration tests and `make ci` passes.

## Requirements

#### Functional Requirements

1.  **Frontend Dependency Refactoring, Autoprefixer & PostCSS Removal:**
    *   The `frontend/package.json` shall be refactored.
    *   The `@playwright/test` devDependency shall be removed.
    *   The `autoprefixer` devDependency shall be removed.
    *   The `postcss` devDependency shall be removed.
    *   The `frontend/postcss.config.js` file shall be removed.
    *   The following devDependencies shall be moved to runtime `dependencies`: `bits-ui`, `clsx`, `mode-watcher`, `svelte-sonner`, `tailwind-merge`, `tailwind-variants`.
    *   A new Makefile target `front-npm-uninstall` shall be added to `docker/makefiles/frontend.mk` to execute `npm uninstall $(ARGS)` within the `frontend` directory context.
2.  **Temporary Removal of E2E Testing & Documentation Update:**
    *   All files and directories related to Playwright E2E tests currently in `frontend/e2e/` (including `*.test.ts`, `fixtures.ts`) and `frontend/playwright.config.ts` shall be removed.
    *   The `test:e2e` script in `frontend/package.json` shall be removed.
    *   The main `test` script in `frontend/package.json` shall be updated to only run unit tests (e.g., `npm run test:unit -- --run`).
    *   Any Makefile targets specifically for the current E2E tests shall be removed.
    *   `ARCHITECTURE.md` Section 2 (Frontend Architecture) shall be updated to remove references to `frontend/e2e/` and current Playwright setup.
    *   `ARCHITECTURE.md` shall have a new top-level section added (e.g., "10. End-to-End Testing (Future Phase)") stating: "End-to-End (E2E) testing is planned for a future phase. This will likely involve a dedicated E2E testing setup, potentially located in a root `mus/e2e/` directory, focusing on testing the application as deployed in the production Docker container."
3.  **Simplified Track Listing:**
    *   The `GET /api/v1/tracks` API endpoint shall return `List[TrackDTO]` containing all tracks, without pagination.
    *   The `PagedResponseDTO` class and its usage for track listing shall be removed from the backend codebase.
    *   The frontend shall be updated to consume this non-paginated list of tracks.
4.  **Conditional CORS:**
    *   The FastAPI backend shall apply CORS middleware (allowing `http://localhost:5173` and standard permissive methods/headers) if the `APP_ENV` environment variable is *not* equal to "production".
    *   If `APP_ENV` is "production", CORS middleware shall *not* be applied.
5.  **Development Startup Routine:**
    *   When the FastAPI application starts:
        *   The SQLite database shall be completely reset: all tables dropped and then recreated based on `SQLModel.metadata`. This operation must use the existing asynchronous engine.
        *   The contents of the `./data/covers` directory shall be removed.
        *   The `ScanTracksUseCase.scan_directory()` method shall be invoked asynchronously to start scanning for music. This process must not block the server from becoming ready to handle requests.
6.  **Configurable Music Directory:**
    *   The `FileSystemScanner` shall use the path specified in the `MUSIC_DIR` environment variable as the root for music scanning.
    *   If the `MUSIC_DIR` environment variable is not set, the scanner shall default to `./music`.
    *   The specified `MUSIC_DIR` (or default) should be created if it doesn't exist during scanner initialization.

#### Technical Requirements

*   Backend: Python 3.12+, FastAPI, SQLModel, Uvicorn.
*   Frontend: SvelteKit, TypeScript, npm, Tailwind CSS via `@tailwindcss/vite`.
*   Environment Variables: `APP_ENV` (for CORS and startup routine), `MUSIC_DIR`.
*   Database Operations: Use `SQLModel.metadata.drop_all/create_all` with `conn.run_sync()` on an `AsyncConnection` from the async engine for DB reset.
*   Asynchronous Operations: Startup scan must be non-blocking.
*   Build tools (`svelte`, `@sveltejs/kit`, `vite`, `typescript`, `tailwindcss`) remain in `devDependencies`. Runtime utilities (`date-fns`, `lucide-svelte`, `bits-ui`, `clsx`, etc.) must be in `dependencies`.

## Development Details

*   **New Dependencies:** None anticipated. Dependencies will be re-categorized or removed.
*   **Prerequisite Changes:**
    *   Understanding of FastAPI `lifespan` events.
    *   Familiarity with SQLAlchemy/SQLModel DDL operations with an async engine.
    *   Understanding of `npm` dependency management (`dependencies` vs `devDependencies`).
    *   Confirmation that `@tailwindcss/vite` handles PostCSS needs implicitly.
*   **Relevant Files:**
    *   `docker/makefiles/frontend.mk` (new `front-npm-uninstall` target, usage of `front-npm-install`)
    *   `frontend/package.json` (dependency changes, script updates, `postcss` removal)
    *   `frontend/package-lock.json` (updated by npm)
    *   `frontend/src/app.css` (checked for non-standard PostCSS syntax)
    *   `frontend/e2e/` (to be removed)
    *   `frontend/playwright.config.ts` (to be removed)
    *   `ARCHITECTURE.md` (E2E section update, PostCSS removal)
    *   `README.md` (PostCSS removal from tech stack)
    *   `backend/src/mus/main.py` (CORS, lifespan)
    *   `backend/src/mus/infrastructure/database.py` (Engine usage)
    *   `backend/src/mus/infrastructure/scanner/file_system_scanner.py` (`MUSIC_DIR`)
    *   `backend/src/mus/application/use_cases/scan_tracks_use_case.py` (Invoked in lifespan)
    *   `backend/src/mus/infrastructure/api/routers/track_router.py` (Track listing)
    *   `backend/src/mus/infrastructure/persistence/sqlite_track_repository.py` (Track listing)
    *   `backend/src/mus/application/dtos/responses.py` (PagedResponseDTO removal)
    *   `backend/src/mus/application/dtos/__init__.py` (PagedResponseDTO removal)
    *   `backend/src/mus/infrastructure/api/schemas.py` (PagedResponseDTO removal)
    *   `frontend/src/lib/services/apiClient.ts` (Track listing)
    *   `frontend/src/routes/(app)/+layout.server.ts` (Track listing)
    *   Associated test files for all modified backend and frontend modules.
*   **Implementation Considerations:**
    *   Ensure robust error handling for directory cleaning and DB operations in the startup routine.
    *   Manually instantiate `ScanTracksUseCase` and its dependencies within the `lifespan` context, as FastAPI's `Depends` won't work directly there. A session from `get_session_generator` will be needed.
    *   The user's specific path (`/Users/mayurifag/Nextcloud/Music`) should be set via the `MUSIC_DIR` environment variable in their local development setup (e.g., `.env` file or Makefile target), not hardcoded in the application.
    *   Carefully manage `npm uninstall` and `npm install` sequences to correctly move dependencies and remove PostCSS.

## Testing Approach

*   **Frontend Dependency Refactoring & Autoprefixer/PostCSS Removal:**
    *   Verify `@playwright/test`, `autoprefixer`, and `postcss` are removed from `frontend/package.json`.
    *   Verify specified packages (`bits-ui`, `clsx`, etc.) are moved from `devDependencies` to `dependencies` in `frontend/package.json`.
    *   Verify `frontend/postcss.config.js` is deleted.
    *   Verify `front-npm-uninstall` target exists and functions correctly in `docker/makefiles/frontend.mk`.
    *   Verify `make ci` passes after all changes, ensuring frontend build, linting, and tests are successful and styles are correctly applied.
*   **E2E Test Removal & ARCHITECTURE.md Update:** Verify removal of specified files and script updates. Verify `ARCHITECTURE.md` reflects the changes.
*   **Simplified Track Listing & PagedResponseDTO Removal:**
    *   Unit tests for `SQLiteTrackRepository` to ensure `get_all()` fetches all tracks without pagination.
    *   Integration tests for `GET /api/v1/tracks` to verify it returns `List[TrackDTO]`.
    *   Verify `PagedResponseDTO` is completely removed from DTOs, schemas.
    *   Frontend: Unit tests for `apiClient.ts` (`fetchTracks`) and `+layout.server.ts` (if applicable) to ensure they handle `List[TrackDTO]`.
*   **Conditional CORS:**
    *   Integration tests for the backend. Set `APP_ENV` to "development" (or unset) and "production" respectively.
    *   Verify presence of `Access-Control-Allow-Origin: http://localhost:5173` (and other relevant CORS headers) in "development" via an OPTIONS request from a test origin.
    *   Verify absence of CORS headers in "production".
*   **Development Startup Process:**
    *   Unit/integration tests for the DB reset logic within the `lifespan` manager (mocking DDL calls on `conn.run_sync`).
    *   Unit tests for the cover directory cleaning logic.
    *   Integration tests for the `lifespan` manager to ensure it correctly identifies non-production environment (via `APP_ENV`) and triggers:
        *   DB reset calls.
        *   Cover cleaning calls.
        *   Asynchronous invocation of `ScanTracksUseCase.scan_directory()` (mock `ScanTracksUseCase` and `asyncio.create_task`).
*   **`MUSIC_DIR` Configuration:**
    *   Unit tests for `FileSystemScanner` to verify it reads `MUSIC_DIR` correctly and defaults to `./music` if unset.
    *   Integration tests (potentially as part of scan tests) verifying that the scan process targets the correct directory based on a set `MUSIC_DIR` and the default.
