# Mus Next Generation Architecture

## Overview

Mus Next Generation is a modern rewrite of the Mus personal music server, featuring a universal desktop/web application architecture. It leverages Tauri 2.0 for desktop, Vite/React/TypeScript for the frontend, and FastAPI/SQLModel for the backend, all containerized with Docker.

## Frontend Architecture

- **Framework:** Vite + React + TypeScript
- **State Management:** Zustand
- **Styling:** Tailwind CSS, Shadcn/UI
- **Data Fetching:** TanStack Query
- **PWA:** Service worker for offline shell
- **Directory Structure:**
  - `src/pages/`, `src/components/`, `src/hooks/`, `src/services/`, `src/store/`, `src/types/`, `src/utils/`, `src/assets/`

## Backend Architecture

- **Framework:** FastAPI
- **Database:** SQLite via SQLModel (async)
- **Architecture:** Hexagonal (domain, application, infrastructure)
- **Authentication:** JWT (python-jose), cookie-based for web
- **Testing:** Pytest, HTTPX, pytest-asyncio
- **Image Processing:** pyvips
- **Directory Structure:**
  - `src/mus/domain/`, `src/mus/application/`, `src/mus/infrastructure/`

## Tauri Integration

- **Purpose:** Wraps frontend as a universal desktop app
- **Config:** `tauri/tauri.conf.json` (or TOML/JSON5)
- **Features:** Window management, future native integrations (media keys, notifications)

## Docker Setup

- **Dockerfiles:**
  - `docker/backend.Dockerfile` (backend dev/CI)
  - `docker/frontend.Dockerfile` (frontend dev/CI)
  - `docker/production.Dockerfile` (multi-stage, production)
- **Compose:** `docker-compose.yml` for local orchestration
- **Makefiles:** Granular targets in `docker/makefiles/`

## Data Flow

- **Frontend <-> Backend:** JSON REST API
- **Backend <-> DB:** SQLModel ORM (async)
- **Tauri <-> Backend:** Direct API calls (no JWT required)
- **Web Auth:** Secret key login, JWT cookie for browser

## Authentication System

The authentication system is specifically designed for web access, while Tauri desktop access bypasses authentication requirements. This dual approach allows for a secure web interface while maintaining a seamless desktop experience.

### Web Authentication Flow

1. **Initial Authentication:**
   - User accesses: `/api/v1/auth/login-via-secret/{secret_key}`
   - If valid, a JWT token is generated with `sub`, `exp`, and `iat` claims
   - Token is set as an HttpOnly cookie named `mus_auth_token`
   - User is redirected to the root path (`/`)

2. **Subsequent Requests:**
   - JWT cookie is automatically sent with each request
   - Protected endpoints use the `get_current_user` dependency
   - Token is validated and decoded
   - Request proceeds if token is valid, otherwise 401 Unauthorized

3. **Security Considerations:**
   - JWT signing key is set via `AUTH_SECRET_KEY` environment variable
   - Tokens expire after 24 hours by default (configurable)
   - HttpOnly cookie prevents JavaScript access
   - No JWT protection for API endpoints accessed by Tauri

### Technical Implementation

- **JWT Library:** python-jose for token generation and validation
- **Algorithm:** HS256 (HMAC with SHA-256)
- **Cookie Settings:** HttpOnly, Secure, SameSite=Lax
- **Configuration:** All auth settings in `config.py` with environment variable overrides
