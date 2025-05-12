---
id: R3W7
type: feature
description: Full project rewrite to Tauri 2.0, SvelteKit/TS frontend, FastAPI/SQLModel backend.
---
## Project Rewrite: Mus Next Generation (SvelteKit Frontend)

**This Product Requirements Document (PRD) outlines the plan for a complete rewrite of the Mus project, now targeting SvelteKit for the frontend. The AI agent will generate actual code based on the "Moves" associated with this PRD. Full code for new/modified files will be provided by the agent during the execution of each Move.**

### 1. Feature Overview

*   **Feature Name and ID:** Mus Project Rewrite (R3W7)
*   **Purpose Statement:** To modernize the Mus personal music server by rebuilding it with a contemporary technology stack, including a universal desktop application layer with Tauri 2.0, a rich client-side frontend using SvelteKit/TypeScript, and a refined FastAPI backend with an async SQLModel/SQLite database. The rewrite aims to enhance user experience, maintainability, security, developer productivity, and leverage SvelteKit's performance characteristics.
*   **Problem Being Solved:**
    *   Outdated frontend technology (HTMX, vanilla JS) limiting UI/UX capabilities.
    *   Desire for a native desktop application experience.
    *   Need for a more robust and clearly structured backend API.
    *   Opportunity to implement a more standard and secure authentication mechanism for *web access*.
    *   Improve overall project structure, tooling, and maintainability.
    *   Leverage Svelte's compile-time approach for potentially better frontend performance.
*   **Success Metrics:**
    *   Successful build and packaging of a universal desktop application (Tauri 2.0) for at least one major OS.
    *   Fully functional frontend (SvelteKit/TS) replicating and enhancing core music playback features.
    *   Backend API (FastAPI/SQLModel) successfully serving the frontend and Tauri app.
    *   Authentication for web access functioning as specified.
    *   Project structure implemented as per the new layout (`frontend/`, `backend/`, `docker/`).
    *   All core features from the previous version (playback, browsing, state persistence, scanning) are present and working.
    *   CI/CD pipelines updated and functional for the new structure and SvelteKit frontend.

### 2. Requirements

#### 2.1. Functional Requirements

*   **Core Music Playback:**
    *   Browse and list all scanned music tracks.
    *   Play, pause, skip to next/previous track.
    *   Display track metadata (title, artist, duration, cover art).
    *   Volume control and mute functionality.
    *   Seekable progress bar.
    *   Player state (current track, progress, volume, mute) persistence across sessions (via backend).
*   **Track Management (Backend):**
    *   Scan specified music directories for new/updated tracks (UPSERT logic).
    *   Extract basic metadata (title, artist, duration - using filename initially, potentially enhanced later).
    *   Extract cover art from audio files (if possible) or use embedded.
    *   Process cover art into 'original' and 'small' (e.g., 80x80) WebP formats.
    *   Store track information and cover status in a SQLite database.
    *   Efficiently serve track data, audio streams (chunked/ranged), and cover art to the frontend.
*   **Authentication (Web Access Only):**
    *   User navigates to `GET /api/v1/auth/login-via-secret/{YOUR_SECRET_KEY_IN_PATH}` in a web browser.
    *   Backend validates the secret key (from environment variable).
    *   If valid, backend generates a JWT and sets an HttpOnly, Secure (in prod), SameSite=Lax cookie (`mus_auth_token`).
    *   Backend redirects to the frontend's main page (`/`).
    *   Subsequent API requests *from the web browser* automatically include this cookie.
    *   The Tauri application will *not* use this secret key flow and will access the API directly without JWT authentication.
*   **Frontend Application (SvelteKit):**
    *   Single Page Application (SPA) experience.
    *   Responsive design.
    *   Dark theme by default.
    *   PWA capabilities: app shell, metadata caching (for offline app shell availability).
*   **Tauri Desktop Application:**
    *   Wrap the SvelteKit frontend build output into a universal desktop application.
    *   Basic native integration (e.g., window management).
*   **API:**
    *   JSON RESTful API for communication between frontend/Tauri and backend. Endpoints detailed in `ARCHITECTURE.md`.

#### 2.2. Technical Requirements

*   **Project Structure:**
    *   Root directories: `backend/`, `frontend/`, `docker/`.
    *   Root files: `.editorconfig`, `.gitignore` (root and per-service), `Makefile`, `README.md`, `ARCHITECTURE.md`.
*   **Frontend:**
    *   Stack: SvelteKit, TypeScript.
    *   State Management: Svelte Stores (`writable`, `readable`, `derived`).
    *   Styling: Tailwind CSS.
    *   UI Components: `shadcn-svelte`.
    *   Icons: `lucide-svelte`.
    *   Data Fetching: SvelteKit `load` functions, `fetch` API.
    *   Date/Time: `date-fns`.
    *   Build Tool: SvelteKit (via Vite).
    *   Package Manager: `npm`.
*   **Backend:**
    *   Stack: FastAPI, Python 3.12+.
    *   Package Manager: `uv`.
    *   Database: SQLite with SQLModel (async).
    *   API: JSON RESTful.
    *   Architecture: Hexagonal (`domain/`, `application/`, `infrastructure/`).
    *   Testing: Pytest, `pytest-asyncio`, HTTPX via `TestClient`.
    *   Libraries: `python-jose` for JWTs (HS256 algorithm), `pyvips` for cover processing.
*   **Tauri 2.0:**
    *   Setup to package the SvelteKit frontend build output (from `frontend/build`).
    *   Configuration via `frontend/src-tauri/tauri.conf.json` (or alternative).
*   **Docker:**
    *   `docker/backend.Dockerfile`: For backend development/CI.
    *   `docker/production.Dockerfile`: Multi-stage build (builds SvelteKit frontend, combines with backend).
    *   `docker-compose.yml`: For local development orchestration.
*   **Makefiles:**
    *   Root `Makefile` includes `docker/makefiles/*.mk`.
    *   Targets for install, dev, build, lint, test for frontend, backend, Tauri, and Docker.
*   **Security:**
    *   HttpOnly, Secure, SameSite=Lax cookies for web authentication JWT.
    *   Backend API endpoints (`/api/v1/tracks`, `/api/v1/tracks/.../stream`, `/api/v1/tracks/.../covers/...`, `/api/v1/scan`, `/api/v1/player/state`) accessible *without* JWT authentication (for Tauri and simple web access after initial gate). The secret key flow is the gate for web browser access to the *frontend*, not directly protecting most API data endpoints.
*   **Performance:**
    *   Efficient track scanning and metadata/cover extraction.
    *   Responsive frontend UI leveraging Svelte's compiled nature.
    *   Optimized Docker images.
    *   Chunked audio streaming from backend.

#### 2.3. User Requirements (Implicit)

*   Intuitive music playback interface.
*   Straightforward setup/installation (especially desktop app).
*   Stable and reliable operation.

#### 2.4. Constraints

*   `bun` is not to be used for frontend build tooling or package management; `npm` is preferred for frontend, `uv` for backend.
*   Stick to the specified libraries unless a strong justification arises.
*   Minimal comments in code, favoring self-documenting names and structure.

### 3. Design and Implementation

*(High-level architecture, specific component structures, API endpoints, Data Models, and detailed Authentication Flow are now primarily documented in `ARCHITECTURE.md`)*

*   **Frontend Architecture:** See `ARCHITECTURE.md` Section 2. Uses SvelteKit conventions (`src/routes`, `src/lib`).
*   **Backend Architecture:** See `ARCHITECTURE.md` Section 3. Follows Hexagonal Architecture.
*   **Tauri Setup:** See `ARCHITECTURE.md` Section 4. Configured in `frontend/src-tauri/`.
*   **API Endpoints:** Defined in `ARCHITECTURE.md` Section 7 & 3.
*   **Data Models:** Defined in `ARCHITECTURE.md` Section 8 (`Track`, `PlayerState`).
*   **Authentication Flow (Web):** Detailed in `ARCHITECTURE.md` Section 7.

### 4. Development Details

*   **New Dependencies:**
    *   **Frontend:** `@sveltejs/kit`, `svelte`, `@sveltejs/adapter-auto`, `vite`, `typescript`, `tailwindcss`, `postcss`, `autoprefixer`, `shadcn-svelte`, `bits-ui`, `clsx`, `tailwind-variants`, `tailwind-merge`, `lucide-svelte`, `date-fns`, `@testing-library/svelte`, `vitest`, `jsdom`, `msw`.
    *   **Backend:** (Existing from previous moves) `fastapi`, `sqlmodel`, `uvicorn`, `python-jose[cryptography]`, `pytest`, `pytest-asyncio`, `httpx`, `aiosqlite`, `pyvips`.
    *   **Tauri:** `@tauri-apps/cli`, `@tauri-apps/api`.
*   **Prerequisite Changes:**
    *   Complete removal/replacement of old frontend (HTMX/JS/CSS).
    *   Creation of new `frontend/` directory using SvelteKit template.
    *   Implementation/Refinement of backend API, persistence (SQLModel), scanning logic.
    *   New `ARCHITECTURE.md`.
    *   Updated Makefiles and Dockerfiles for SvelteKit.
    *   Updated CI/CD configuration in `.github/workflows/` for SvelteKit.
*   **Implementation Considerations:**
    *   Phased approach: Finalize Backend core functionality & tests -> Frontend setup & core -> Frontend features -> Tauri integration -> Docker & CI refinement.
    *   Leverage SvelteKit's `load` functions for efficient data loading.
    *   Manage state effectively using Svelte Stores.
    *   Ensure Tailwind CSS purging is correctly configured in SvelteKit.
    *   Configure Vite/SvelteKit for optimal build and development.
    *   Handle Tauri build process and frontend communication needs.
*   **Security Considerations:**
    *   Secure JWT handling for web access cookie.
    *   Environment variables for all secrets.
    *   Input validation for API endpoints (via FastAPI/Pydantic).
    *   Regular dependency updates (frontend and backend).

### 5. Testing Approach

*   **Backend:**
    *   Unit tests for domain logic, use cases (Pytest).
    *   Integration tests for API endpoints using FastAPI's `TestClient` (HTTPX) and test DB (Pytest).
    *   Focus on testing SQLModel repository interactions (UPSERTs).
    *   Aim for >80% test coverage.
*   **Frontend:**
    *   Unit tests for components, stores, utils using Vitest and `@testing-library/svelte`.
    *   Integration tests for page flows using Vitest and potentially MSW for mocking API/`load` functions.
    *   Aim for >70% test coverage.
*   **Tauri:**
    *   Manual testing of interaction between frontend and backend within the Tauri shell.
    *   Testing build and packaging process.

### 6. Design Assets (Conceptual)

*   Guided by `shadcn-svelte` components and Tailwind CSS.
*   Clean, modern, dark-themed interface.
*   Focus on usability for music browsing and playback.

### 7. Future Considerations

*   Advanced PWA features (offline audio).
*   Deeper Tauri native integrations (notifications, media keys).
*   Theme switcher.
*   Playlist management.
*   User accounts (beyond secret key).
*   Advanced search/filtering.

### 8. What to Avoid

*   Over-complicating the initial rewrite scope.
*   Introducing libraries not specified without strong reason.
*   Deviating from chosen architectures (Hexagonal, SvelteKit patterns).
*   Premature optimization.

This PRD provides the foundation for the Mus project rewrite using SvelteKit. The detailed tasks are broken down in the `R3W7-moves.json` file.
