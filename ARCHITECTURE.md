# Mus Next Generation Architecture

## 1. Overview

Mus Next Generation is a modern rewrite of the Mus personal music server. It provides a unified experience across web, with desktop integration (via Tauri) planned for a future phase.

The core architecture consists of:

*   **Frontend:** A Single Page Application (SPA) built with SvelteKit and TypeScript, styled with Tailwind CSS and `shadcn-svelte`.
*   **Backend:** A RESTful API server built with FastAPI (Python 3.12+) using asynchronous operations, SQLModel for database interaction (SQLite), and following Hexagonal Architecture principles.
*   **Desktop Shell:** *(Future Phase)* Tauri 2.0 will wrap the SvelteKit frontend, providing a native desktop application experience.
*   **Containerization:** Docker is used for development, testing, and production deployment, orchestrated with Docker Compose for local development.
*   **Makefiles:** Located in `makefiles/`. Provide convenient targets (`make build`, `make up`, `make lint`, `make test`, etc.) abstracting Docker and service-specific commands. The root `Makefile` includes these.

## 2. Frontend Architecture (SvelteKit)

*   **Framework:** SvelteKit with TypeScript. Svelte compiles components to efficient, imperative JavaScript at build time, reducing framework runtime overhead compared to virtual DOM libraries.
*   **Build & Runtime:** Uses Vite for development and bundling. Configured with `adapter-auto` for flexible deployment (Node server, serverless, or potentially static with fallbacks), aiming for an SPA-like experience for dynamic content.
*   **Routing:** File-based routing provided by SvelteKit (`src/routes`).
*   **State Management:** Primarily uses Svelte's built-in Stores (`writable`, `readable`, `derived`) located in `src/lib/stores`.
*   **Styling:** Tailwind CSS utility-first framework, configured via `tailwind.config.js`. `clsx` and `tailwind-merge` for utility class composition.
*   **UI Components:** `shadcn-svelte` component library, built on top of unstyled primitives and Tailwind CSS. Custom components are organized within `src/lib/components`.
*   **Icons:** `lucide-svelte` for icons.
*   **Data Fetching:**
    *   Initial page/layout data fetching uses SvelteKit's `load` functions (`+page.server.js`, `+layout.server.js`).
    *   Client-side data fetching and mutations (e.g., saving player state, triggering scans) use the native `fetch` API, potentially wrapped in utility functions in `src/lib/services/apiClient.ts`.
*   **Date/Time:** `date-fns` for date/time formatting and manipulation.
*   **PWA:** Basic Progressive Web App features (manifest, service worker for app shell caching) implemented for web deployment.
*   **Directory Structure (`frontend/`):**
    *   `src/`: Main application code.
        *   `lib/`: Reusable code (components, stores, services, types, utils).
            *   `components/`: Svelte components (UI primitives from `shadcn-svelte`, layout, domain-specific).
            *   `stores/`: Svelte stores (e.g., `playerStore.ts`, `trackStore.ts`).
            *   `services/`: API client logic, utility functions interacting with the backend.
            *   `types/`: TypeScript type definitions.
            *   `utils/`: General utility functions (e.g., date formatting).
        *   `routes/`: Application pages and API routes (file-based routing). Contains `.svelte` files for pages/layouts and `+page.server.ts`/`+layout.server.ts` for `load` functions.
        *   `app.html`: Main HTML template.
        *   `hooks.server.ts`: Server-side hooks.
        *   `hooks.client.ts`: Client-side hooks (e.g., service worker registration).
        *   `service-worker.ts`: Service worker logic.
    *   `static/`: Static assets (images, fonts, `manifest.json`).
    *   `tests/`: Vitest unit tests.
    *   `src-tauri/`: Tauri specific configuration and Rust code (if any).
    *   `svelte.config.js`, `vite.config.ts`, `tailwind.config.js`, `postcss.config.js`, `tsconfig.json`, `.eslintrc.cjs`, `.prettierrc`.

## 3. Backend Architecture (FastAPI)

*   **Framework:** FastAPI (Python 3.12+), asynchronous using `async`/`await`.
*   **Package Management:** `uv` for dependency management and execution environment.
*   **Database:** SQLite accessed asynchronously via SQLModel ORM.
*   **API:** JSON RESTful API served over HTTP.
*   **Architecture:** Hexagonal Architecture.
    *   `src/mus/domain/`: Core business logic and entities (e.g., `Track`, `PlayerState` SQLModels). Independent of infrastructure.
    *   `src/mus/application/`: Use cases orchestrating domain logic (e.g., `ScanTracksUseCase`, `ManagePlayerStateUseCase`) and Data Transfer Objects (DTOs).
    *   `src/mus/infrastructure/`: Implementation details.
        *   `api/`: FastAPI routers, request/response models (schemas), dependencies.
        *   `persistence/`: Database repository implementations (e.g., `SQLiteTrackRepository`).
        *   `scanner/`: File system scanning, metadata extraction (using external libraries like `pyvips` for covers), cover processing logic.
        *   `auth/`: JWT generation/validation logic for web access.
*   **Configuration:** Managed via Pydantic models (`src/mus/config.py`) loading from environment variables (`.env` file for development). Key paths (DB, music dir, covers dir) are hardcoded for simplicity in current iteration but managed via `os.makedirs`.
*   **Testing:** `pytest` with `pytest-asyncio` for async tests, `httpx` via FastAPI's `TestClient` for API integration tests.
*   **Directory Structure (`backend/`):**
    *   `src/mus/`: Main application package.
        *   `domain/`, `application/`, `infrastructure/` (as described above).
        *   `main.py`: FastAPI app instantiation, lifespan events, root router includes.
        *   `config.py`: Configuration settings.
        *   `database.py`: Database engine setup, session management.
    *   `tests/`: Pytest test suite.
    *   `pyproject.toml`, `requirements.txt` (generated by `uv pip compile`).

## 4. Tauri Integration (Planned for Future Phase)

*   **Status:** Deferred to a future development phase. The current implementation focuses on the web interface.
*   **Purpose:** Will package the SvelteKit frontend into a cross-platform desktop application using system webviews.
*   **Configuration:** Will be managed via `frontend/src-tauri/tauri.conf.json` (or `Tauri.toml`/`.json5`). Will specify window parameters, build settings (pointing to SvelteKit's `build` directory), and the allowlist for native API access.
*   **Communication:** The SvelteKit frontend running inside Tauri will communicate directly with the FastAPI backend via HTTP requests to its API endpoints (e.g., `http://localhost:8000/api/v1/...`). Tauri itself won't mediate API calls unless specific Tauri commands are implemented for native functionality.
*   **API Access:** Backend API endpoints intended for Tauri access will *not* require the JWT authentication used for web browser access.
*   **Future Considerations:** Potential native integrations include media key support, native notifications for scan completion, and potentially using Tauri's file system APIs.

## 5. Docker Setup

*   **Purpose:** Provide consistent development, testing, and production environments.
*   **Files:**
    *   `docker/backend.Dockerfile`: Builds a development/CI image for the backend using `uv`.
    *   `docker/production.Dockerfile`: Multi-stage build. Stage 1 builds the SvelteKit frontend assets (`npm run build`). Stage 2 sets up the Python environment, installs backend dependencies, copies the backend code, and copies the built frontend assets from Stage 1 into a location served by FastAPI (`/app/backend/static_root`). FastAPI is configured to serve these static assets.
    *   `docker-compose.yml`: Orchestrates local development, defining services for the backend and potentially the frontend dev server (though often frontend dev is run locally), mounting volumes for code, data, and music.

## 6. Data Flow & Interaction

*   **SvelteKit Frontend (Web) <-> FastAPI Backend:** Communication via JSON REST API calls over HTTP (e.g., `GET /api/v1/tracks`, `POST /api/v1/player/state`).
*   **FastAPI Backend <-> SQLite Database:** Asynchronous interaction via SQLModel ORM.
*   **User (Web Browser) -> FastAPI Backend (Auth):** Initial authentication via `GET /api/v1/auth/login-via-secret/{key}`, setting an HttpOnly JWT cookie. Subsequent web requests include the cookie.
*   **User (Tauri App) -> FastAPI Backend:** *(Future Phase)* Will use direct API calls without JWT authentication.

## 7. Authentication System

*   **Mechanism:** A secret key provided in the URL path (`/api/v1/auth/login-via-secret/{SECRET_KEY}`) grants initial access for web browsers.
*   **Process:**
    1.  Backend validates the secret key against an environment variable.
    2.  If valid, a JWT (HS256) is generated containing `sub`, `iat`, `exp` claims, signed with a separate `SECRET_KEY` environment variable.
    3.  The JWT is set as an HttpOnly, Secure (in production), SameSite=Lax cookie (`mus_auth_token`).
    4.  A redirect to the frontend root (`/`) is issued.
*   **Subsequent Web Requests:** The browser automatically sends the cookie. A FastAPI dependency (`get_current_user`) verifies the JWT for protected routes (currently minimal, mainly `/api/v1/me` example).
*   **Tauri Access:** *(Future Phase)* Tauri application will bypass this flow and access API endpoints directly without the JWT cookie. API endpoints crucial for core functionality (`/tracks`, `/stream`, `/covers`, `/scan`, `/player/state`) are *not* protected by JWT validation to support this future integration.

## 8. Data Models

*   **Track (SQLModel):** `id`, `title`, `artist`, `duration` (int, seconds), `file_path` (str, unique), `added_at` (int, timestamp), `has_cover` (bool).
*   **PlayerState (SQLModel):** `id` (fixed to 1), `current_track_id` (Optional[int]), `progress_seconds` (float), `volume_level` (float), `is_muted` (bool). Stored as a single row in the DB using UPSERT.
*   **API DTOs (Pydantic):** Defined in `backend/src/mus/application/dtos/`. Used for API request validation and response serialization. Includes `TrackDTO` which adds derived fields like `cover_small_url` and `cover_original_url`.

## 9. Key Technical Decisions & Considerations

*   **SvelteKit over React:** Chosen for potential performance benefits due to compile-time optimizations and reduced framework runtime, aiming closer to the "no runtime" ideal, though acknowledging JS is still required for SPA interactivity.
*   **Shadcn-Svelte over other UI libs:** Provides unstyled, accessible components using Tailwind, aligning with the chosen styling approach.
*   **SQLModel over SQLAlchemy Core/ORM:** Simplifies Pydantic model and ORM model definition.
*   **Hexagonal Architecture (Backend):** Promotes separation of concerns, testability, and maintainability.
*   **Async Everywhere:** Backend uses `async`/`await` for all I/O bound operations (API handling, DB access) for better concurrency.
*   **Hardcoded Paths (Backend):** Database, music, and covers directories are currently hardcoded in relevant backend modules for simplicity, managed with `os.makedirs`. This could be made configurable via environment variables if needed later.
*   **Web vs. Tauri Auth:** *(Future Phase)* Separate authentication strategy acknowledges the different trust levels and user experience goals for web access vs. the planned desktop application.
*   **Makefile Abstraction:** Simplifies common development tasks (linting, testing, building, running).

## 10. End-to-End Testing (Future)

End-to-End (E2E) testing is planned for a future phase. This will likely involve a dedicated E2E testing setup, potentially located in a root `mus/e2e/` directory, focusing on testing the application as deployed in the production Docker container.

## 10. End-to-End Testing (Future)

End-to-End (E2E) testing is planned for a future phase. This will likely involve a dedicated E2E testing setup, potentially located in a root `mus/e2e/` directory, focusing on testing the application as deployed in the production Docker container.
