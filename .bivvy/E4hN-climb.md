---
id: E4hN
type: feature
description: Enhance backend with configurable music directory, conditional CORS, development startup routines (DB reset, async scan), and simplified track listing.
---

## Feature Overview

*   **Feature Name and ID:** Backend Enhancements and Refinements (E4hN)
*   **Purpose Statement:** To improve the development experience, configurability, and API structure of the Mus backend. This includes making the music source directory configurable, adjusting CORS behavior for development vs. production, automating data reset and scanning on development startup, and simplifying the track listing API.
*   **Problem Being Solved:**
    *   Hardcoded music directory limits development flexibility.
    *   Static CORS policy not ideal for differing dev/prod needs.
    *   Manual data reset and scanning during development is cumbersome.
    *   Paginated track listing API is currently an over-optimization for the frontend's needs.
*   **Success Metrics:**
    *   `GET /api/v1/tracks` returns a simple list of all tracks. `PagedResponseDTO` is removed.
    *   CORS middleware is applied conditionally based on `APP_ENV`.
    *   On development startup (when `APP_ENV` is not "production"), the database is reset, covers folder is cleaned, and music scanning starts asynchronously.
    *   The music directory for scanning is configurable via the `MUSIC_DIR` environment variable, defaulting to `./music`.
    *   All changes are covered by appropriate tests.

## Requirements

#### Functional Requirements

1.  **Simplified Track Listing:**
    *   The `GET /api/v1/tracks` API endpoint shall return `List[TrackDTO]` containing all tracks, without pagination.
    *   The `PagedResponseDTO` class and its usage for track listing shall be removed from the backend codebase.
    *   The frontend shall be updated to consume this non-paginated list of tracks.
2.  **Conditional CORS:**
    *   The FastAPI backend shall apply CORS middleware (allowing `http://localhost:5173` and standard permissive methods/headers) if the `APP_ENV` environment variable is *not* equal to "production".
    *   If `APP_ENV` is "production", CORS middleware shall *not* be applied.
3.  **Development Startup Routine:**
    *   When the FastAPI application starts and `APP_ENV` is *not* "production":
        *   The SQLite database shall be completely reset: all tables dropped and then recreated based on `SQLModel.metadata`. This operation must use the existing asynchronous engine.
        *   The contents of the `./data/covers` directory shall be removed.
        *   The `ScanTracksUseCase.scan_directory()` method shall be invoked asynchronously to start scanning for music. This process must not block the server from becoming ready to handle requests.
4.  **Configurable Music Directory:**
    *   The `FileSystemScanner` shall use the path specified in the `MUSIC_DIR` environment variable as the root for music scanning.
    *   If the `MUSIC_DIR` environment variable is not set, the scanner shall default to `./music`.
    *   The specified `MUSIC_DIR` (or default) should be created if it doesn't exist during scanner initialization.

#### Technical Requirements

*   Backend: Python 3.12+, FastAPI, SQLModel, Uvicorn.
*   Frontend: SvelteKit, TypeScript.
*   Environment Variables: `APP_ENV` (for CORS and startup routine), `MUSIC_DIR`.
*   Database Operations: Use `SQLModel.metadata.drop_all/create_all` with `conn.run_sync()` on an `AsyncConnection` from the async engine for DB reset.
*   Asynchronous Operations: Startup scan must be non-blocking.

## Development Details

*   **New Dependencies:** None anticipated.
*   **Prerequisite Changes:**
    *   Understanding of FastAPI `lifespan` events.
    *   Familiarity with SQLAlchemy/SQLModel DDL operations with an async engine.
*   **Relevant Files:**
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

## Testing Approach

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