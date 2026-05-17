# Project Rules

## Always Use Make

Never run cargo, vitest, docker compose, or npm directly. Use `make` targets — they handle Docker, paths, and env correctly.

Key commands:

- `make up` / `make down` — start/stop dev
- `make logs [service]` — service names: `backend`, `frontend`
- `make ci` — full verification gate (must pass before any work is done)
- `make back-ci` — Rust backend format, check, clippy, tests, dependency audit, and unused dependency check
- `make back-test` / `make front-test` — run tests
- `make back-lint` / `make front-lint` — lint (auto-formats first)
- `make back-sh` — shell into backend container
- `make e2e` — builds prod image, runs Playwright, tears down
- `make e2e-current-image` — runs Playwright against existing `mus:latest` after `make prod-image`
- `make ps` — container status

## Gitignored Override Files

`docker/docker-compose.override.yml` is gitignored but required for dev.
Create from `docker/docker-compose.override.yml.example`.
Contains volume mounts, ports, and service env vars.
If compose fails silently in a fresh checkout, this file is probably missing.
