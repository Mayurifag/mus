# Project Rules

## Always Use Make

Never run pytest, vitest, ruff, docker compose, or npm directly. Use `make` targets — they handle Docker, paths, and env correctly.

Key commands:

- `make up` / `make down` — start/stop dev
- `make logs [service]` — service names: `backend`, `frontend`, `redis`, `streaq-worker`
- `make ci` — full verification gate (must pass before any work is done)
- `make back-test` / `make front-test` — run tests
- `make back-lint` / `make front-lint` — lint (auto-formats first)
- `make back-sh` — shell into backend container
- `make e2e` — builds prod image, runs Playwright, tears down
- `make ps` — container status

## Gitignored Override Files

`docker/docker-compose.override.yml` is gitignored but required for dev.
Create from `docker/docker-compose.override.yml.example`.
Contains volume mounts, ports, env vars including `BACKEND_URL`
(needed for worker→backend SSE).
If compose fails silently in a fresh checkout, this file is probably missing.

`.env` at repo root holds `SECRET_KEY` (also gitignored).
