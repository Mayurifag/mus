---
id: R3W7
type: feature
description: Full project rewrite to Tauri 2.0, Vite/React/TS frontend, FastAPI/SQLModel backend.
---
## Project Rewrite: Mus Next Generation

**This Product Requirements Document (PRD) outlines the plan for a complete rewrite of the Mus project. The AI agent will generate actual code based on the "Moves" associated with this PRD. Full code for new/modified files will be provided by the agent during the execution of each Move, not within this PRD itself.**

### 1. Feature Overview

*   **Feature Name and ID:** Mus Project Rewrite (R3W7)
*   **Purpose Statement:** To modernize the Mus personal music server by rebuilding it with a contemporary technology stack, including a universal desktop application layer with Tauri 2.0, a rich client-side frontend using Vite/React/TypeScript, and a refined FastAPI backend with an async SQLModel/SQLite database. The rewrite aims to enhance user experience, maintainability, security, and developer productivity.
*   **Problem Being Solved:**
    *   Outdated frontend technology (HTMX, vanilla JS) limiting UI/UX capabilities.
    *   Desire for a native desktop application experience.
    *   Need for a more robust and clearly structured backend API.
    *   Opportunity to implement a more standard and secure authentication mechanism for web access.
    *   Improve overall project structure, tooling, and maintainability.
*   **Success Metrics:**
    *   Successful build and packaging of a universal desktop application (Tauri 2.0) for at least one major OS (e.g., Linux, Windows, macOS - to be confirmed by user).
    *   Fully functional frontend (Vite/React/TS) replicating and enhancing core music playback features.
    *   Backend API (FastAPI/SQLModel) successfully serving the frontend and Tauri app.
    *   Authentication for web access functioning as specified.
    *   Project structure implemented as per the new layout (`frontend/`, `backend/`, `docker/`).
    *   All core features from the previous version are present and working in the new version.
    *   CI/CD pipelines updated and functional for the new structure.

### 2. Requirements

#### 2.1. Functional Requirements

*   **Core Music Playback:**
    *   Browse and list all scanned music tracks.
    *   Play, pause, skip to next/previous track.
    *   Display track metadata (title, artist, duration, cover art).
    *   Volume control and mute functionality.
    *   Seekable progress bar.
    *   Player state (current track, progress, volume) persistence across sessions.
*   **Track Management (Backend):**
    *   Scan specified music directories for new tracks.
    *   Extract metadata (title, artist, duration, cover art) from audio files.
    *   Store track information in a SQLite database.
    *   Efficiently serve track data and audio streams to the frontend.
*   **Authentication (Web Access Only):**
    *   User navigates to `GET /api/v1/auth/login-via-secret/{YOUR_SECRET_KEY_IN_PATH}` in a web browser.
    *   Backend validates the secret key.
    *   If valid, backend generates a JWT and sets an HttpOnly, Secure, SameSite=Lax cookie.
    *   Backend redirects to the frontend's main page.
    *   Subsequent API requests *from the web browser* automatically include this cookie.
    *   The Tauri application will *not* use this secret key flow and will access the API directly.
*   **Frontend Application:**
    *   Single Page Application (SPA) experience.
    *   Responsive design.
    *   Dark theme by default.
    *   PWA capabilities: app shell, metadata caching (for offline app shell availability). Limited audio caching can be a future enhancement.
*   **Tauri Desktop Application:**
    *   Wrap the Vite/React frontend into a universal desktop application.
    *   Basic native integration (e.g., window management).
    *   Future considerations (not in initial scope but to keep in mind for architecture): native notifications, media key support.
*   **API:**
    *   JSON RESTful API for communication between frontend/Tauri and backend.

#### 2.2. Technical Requirements

*   **Project Structure:**
    *   Root directories: `backend/`, `frontend/`, `docker/`.
    *   Root files: `.editorconfig`, `.gitignore` (root and per-service), `Makefile`, `README.md`, `ARCHITECTURE.md`.
*   **Frontend:**
    *   Stack: Vite, React, TypeScript.
    *   State Management: Zustand.
    *   Styling: Tailwind CSS (with purging).
    *   UI Components: Shadcn/UI.
    *   Icons: `lucide-react`.
    *   Data Fetching: TanStack Query.
    *   Date/Time: `date-fns`.
    *   Build Tool: `npx create-vite`.
*   **Backend:**
    *   Stack: FastAPI, Python (uv for package management).
    *   Database: SQLite with SQLModel (async).
    *   API: JSON RESTful.
    *   Architecture: Hexagonal (explicit folders: `domain/entities/`, `domain/ports/`, `application/use_cases/`, `infrastructure/api/`, `infrastructure/persistence/`).
    *   Testing: Pytest, `pytest-asyncio`, HTTPX via `TestClient`.
    *   Libraries: `python-jose` for JWTs (HS256 algorithm, `exp`, `iat`, `sub` claims). JWT signing secret as a backend environment variable.
*   **Tauri 2.0:**
    *   Setup to package the Vite/React frontend.
    *   Configuration via `tauri.conf.json` (or `tauri.conf.json5` / `Tauri.toml`).
*   **Docker:**
    *   `docker/backend.Dockerfile`: For backend development/CI.
    *   `docker/frontend.Dockerfile`: For frontend development/CI.
    *   `docker/production.Dockerfile`: Multi-stage build for a lean production image (builds frontend assets, combines with backend).
    *   `docker/makefiles/`: Granular Makefiles.
    *   Docker Compose for local development orchestration.
*   **Makefiles:**
    *   Root `Makefile` includes `docker/makefiles/*.mk`.
    *   Targets for install, dev, build, lint, test for frontend, backend, Tauri, and Docker.
*   **Security:**
    *   HttpOnly, Secure, SameSite=Lax cookies for web authentication.
    *   Backend API endpoints (tracks, stream, state, scan) will be accessible without JWT authentication, particularly for the Tauri app. The secret key flow is a gate for web browser access to the frontend.
*   **Performance:**
    *   Efficient track scanning and metadata extraction.
    *   Responsive frontend UI.
    *   Optimized Docker images.

#### 2.3. User Requirements (Implicit)

*   The application should be intuitive to use for music playback.
*   Setup and installation (especially for the desktop app) should be straightforward.
*   The application should be stable and reliable.

#### 2.4. Constraints

*   `bun` is not to be used for frontend build tooling; `npx` is preferred.
*   Stick to the specified libraries unless a strong justification for alternatives arises.
*   Minimal comments in code, favoring self-documenting names and structure.

### 3. Design and Implementation

#### 3.1. High-Level Architecture
*   **Frontend (Vite/React/TS in `frontend/`):** Handles all UI, user interaction, and client-side state. Communicates with the backend via JSON REST API.
*   **Backend (FastAPI in `backend/`):** Provides the API, manages the music library, handles track scanning, metadata, and streaming.
*   **Tauri Shell:** Wraps the frontend to create a desktop application. Interacts with the backend API.
*   **Docker (`docker/`):** Containerizes frontend (for build), backend, and provides a production-ready image. Docker Compose for local development.

#### 3.2. Frontend Architecture (`frontend/src/`)
*   `main.tsx`: Entry point.
*   `App.tsx`: Root component, routing.
*   `pages/`: Top-level page components (e.g., `HomePage.tsx`).
*   `components/`:
    *   `ui/`: Reusable UI elements from Shadcn/UI or custom.
    *   `layout/`: Structural components (e.g., `PlayerFooter.tsx`, `Sidebar.tsx` - if any).
    *   `domain/`: Feature-specific components (e.g., `TrackList.tsx`, `TrackItem.tsx`).
*   `hooks/`: Custom React hooks (e.g., `usePlayerControls.ts`).
*   `services/`: API communication layer (e.g., `trackService.ts`, `apiClient.ts` using TanStack Query).
*   `store/`: Zustand stores (e.g., `playerStore.ts`).
*   `types/`: TypeScript type definitions (e.g., `track.types.ts`).
*   `utils/`: Utility functions (e.g., `dateUtils.ts` using `date-fns`).
*   `assets/`: Static assets like images, fonts.
*   `service-worker.ts`: For PWA capabilities.
*   `vite.config.ts`, `tailwind.config.js`, `tsconfig.json`, `.eslintrc.js`.

#### 3.3. Backend Architecture (`backend/src/`)
Follows Hexagonal Architecture principles:
*   `mus/` (main package)
    *   `main.py`: FastAPI app instantiation, lifespan events, root router.
    *   `config.py`: Configuration loading (env vars).
    *   `dependencies.py`: FastAPI dependency injection setup.
    *   `domain/`:
        *   `entities/`: Core domain objects (e.g., `track.py` defining `Track` SQLModel).
        *   `ports/`: Interfaces for repositories, external services (e.g., `track_repository.py`).
        *   `services/`: Domain services (if any complex logic not fitting use cases).
    *   `application/`:
        *   `use_cases/`: Application-specific logic orchestrating domain entities and ports (e.g., `scan_tracks_use_case.py`, `manage_player_state_use_case.py`).
        *   `dtos/`: Data Transfer Objects if needed for API boundaries.
    *   `infrastructure/`:
        *   `api/`: FastAPI routers and request/response models (e.g., `track_router.py`, `auth_router.py`).
        *   `persistence/`: Database interaction implementation (e.g., `sqlite_track_repository.py` using SQLModel).
        *   `scanner/`: File system scanning and metadata extraction logic (e.g., `filesystem_scanner.py`, `metadata_extractor_adapter.py` using `mutagen`).
        *   `auth/`: JWT generation/validation logic for web access.
*   `tests/`: Pytest tests.
*   `pyproject.toml`, `requirements.txt` (generated by `uv pip compile`).

#### 3.4. Tauri Setup (`frontend/tauri/`)
*   `tauri.conf.json` (or alternative format): Configures the Tauri application (window settings, permissions, build commands for Vite frontend).
*   Potentially custom Rust code for specific native integrations later on.

#### 3.5. API Endpoints (Backend)
*   `GET /api/v1/auth/login-via-secret/{secret_key}`: Web browser authentication.
*   `GET /api/v1/tracks`: List all tracks.
*   `GET /api/v1/tracks/{track_id}/stream`: Stream audio for a track.
*   `GET /api/v1/tracks/{track_id}/covers/{size}.webp`: Get cover art (`small`, `original`).
*   `POST /api/v1/scan`: Trigger a rescan of the music library.
*   `POST /api/v1/player/state`: Save player state.
*   `GET /api/v1/player/state`: (Primarily used for initial page load for the web frontend, Tauri might fetch differently or use its own persistence).
*   Response/Request models for these endpoints will use Pydantic.

#### 3.6. Data Models
*   **Track (SQLModel):** `id`, `title`, `artist`, `duration`, `file_path` (string), `added_at` (timestamp), `has_cover` (boolean).
*   **PlayerState (SQLModel or JSON file):** `current_track_id`, `progress_seconds`, `volume_level`, `is_muted`.
*   (SQLModel is chosen for database interaction for `Track` and potentially `PlayerState` if stored in DB).

#### 3.7. Authentication Flow (Web)
1.  User accesses `GET /api/v1/auth/login-via-secret/{SECRET_KEY}`.
2.  Backend:
    *   Validates `SECRET_KEY` (from env var).
    *   If invalid, returns 403 Forbidden.
    *   If valid, generates a JWT (`python-jose`).
        *   Payload: `exp` (e.g., 7 days), `iat`, `sub` (e.g., "authenticated-web-user").
        *   Signed with HS256 using a separate backend JWT secret (from env var).
    *   Sets an HttpOnly, Secure, SameSite=Lax cookie (name: `mus_session_jwt`) with the JWT.
    *   Redirects (303 See Other) to the frontend's root URL (e.g., `/`).
3.  Frontend:
    *   Loads. Subsequent API calls to the backend (e.g., `/api/v1/tracks`) will automatically include the `mus_session_jwt` cookie.
4.  Backend API (for web requests):
    *   A middleware will check for the `mus_session_jwt` cookie on relevant API routes (or all API routes if desired for web).
    *   If cookie exists, validate the JWT. If valid, proceed. If invalid/expired, return 401/403.
    *   If cookie doesn't exist, return 401/403.
    *   **Exception:** Tauri app requests to the API will not have/need this JWT. The backend needs to allow these. This could be done by checking a specific header Tauri might send, or by assuming API access without JWT is from Tauri/trusted. For simplicity, the initial plan is that main API routes are not JWT-protected; the `/api/v1/auth/login-via-secret` is the gate for the web frontend.

### 4. Development Details

*   **New Dependencies:**
    *   **Frontend:** `vite`, `react`, `react-dom`, `typescript`, `@types/react`, `@types/react-dom`, `zustand`, `tailwindcss`, `shadcn-ui` (and its dependencies like `lucide-react`, Radix UI components), `tanstack-query`, `date-fns`.
    *   **Backend:** `sqlmodel`, `python-jose[cryptography]`, `uvicorn[standard]`, `fastapi`, `pytest-asyncio`, `httpx`. (uv is the tool, not a library dependency).
    *   **Tauri:** `@tauri-apps/cli`, `@tauri-apps/api`.
*   **Prerequisite Changes:**
    *   Complete removal/replacement of old `src/mus/infrastructure/web/static/` (JS, CSS, templates).
    *   Rewrite of `src/mus/infrastructure/web/main.py` and related files for new API and auth.
    *   Adaptation of `src/mus/infrastructure/persistence/sqlite_track_repository.py` to use SQLModel.
    *   New project structure.
    *   New Makefiles and Dockerfiles.
    *   New CI/CD configuration in `.github/workflows/` to match new structure and tools.
*   **Relevant Files (High-Level - many will be new):**
    *   `frontend/` (entire new directory structure)
    *   `backend/` (significant refactor, new SQLModel entities, hexagonal structure)
    *   `docker/` (new Dockerfiles, makefiles)
    *   `Makefile` (root)
    *   `.github/workflows/deploy.yml`, `.github/workflows/linters.yml`
    *   `README.md`, `ARCHITECTURE.md` (new)
    *   Configuration files: `.editorconfig`, `.gitignore`s.
*   **Implementation Considerations:**
    *   Phased approach: Backend API & DB -> Frontend core -> Tauri integration.
    *   Careful state management with Zustand and data fetching with TanStack Query.
    *   Ensuring async operations are handled correctly in the backend with SQLModel.
    *   Setting up Tailwind CSS purging correctly.
    *   Configuring Vite for optimal build and development.
    *   Tauri build process and frontend communication.
*   **Security Considerations:**
    *   Secure JWT handling for web access.
    *   Environment variables for all secrets and sensitive configurations.
    *   Input validation for API endpoints.
    *   Regular dependency updates.

### 5. Testing Approach

*   **Backend:**
    *   Unit tests for domain logic, use cases.
    *   Integration tests for API endpoints using FastAPI's `TestClient` (HTTPX) and an in-memory SQLite DB or test DB file.
    *   Focus on testing SQLModel repository interactions.
    *   Aim for >80% test coverage.
*   **Frontend:**
    *   Unit tests for components, hooks, utils (e.g., using Vitest or Jest with Testing Library).
    *   Integration tests for page flows and component interactions.
    *   End-to-end tests (e.g., using Playwright or Cypress) for critical user paths (can be a later phase).
*   **Tauri:**
    *   Testing interaction between frontend and Tauri specific APIs (if any are used).
    *   Testing build and packaging process.

### 6. Design Assets (Conceptual)

*   The design will be guided by Shadcn/UI components and Tailwind CSS utility-first principles.
*   A clean, modern, dark-themed interface.
*   Focus on usability and clear information hierarchy for music browsing and playback.

### 7. Future Considerations

*   Advanced PWA features (full offline audio caching, background sync).
*   Deeper Tauri native integrations (notifications, media keys, file system dialogs for library management).
*   Theme switcher (light/dark).
*   Playlist management.
*   User accounts (if moving beyond secret key access).
*   More sophisticated search/filtering.

### 8. What to Avoid

*   Over-complicating the initial rewrite scope. Focus on core functionality first.
*   Introducing unnecessary libraries not listed in the requirements.
*   Deviating from the chosen architectural patterns (Hexagonal, standard React patterns) without strong reason.
*   Premature optimization.

This PRD provides the foundation for the Mus project rewrite. The detailed tasks will be broken down in the `R3W7-moves.json` file.
Use code with caution.
Text
