# Agent Notes

- Always use `make` targets. Do not run `pytest`, `vitest`, `ruff`, `docker compose`, or `npm` directly.
- Main targets: `make up`, `make down`, `make logs [backend|frontend|redis|streaq-worker]`, `make ci`, `make back-test`, `make front-test`, `make back-lint`, `make front-lint`, `make back-sh`, `make e2e`, `make ps`.
- Use `make rebuild-backend-image` to refresh baked backend tools like yt-dlp without deleting volumes. Use `make prod-image` to rebuild the production image locally.
- `docker/docker-compose.override.yml` is gitignored and required for local dev. Create it from `docker/docker-compose.override.yml.example` when missing.
- Local music mount is `/Volumes/sdcard-apfs/OpenCloud/Personal/Music:/app_data/music`.
- Keep `BACKEND_URL` in backend/worker env; worker-to-backend SSE depends on it.
- Streaq v6 worker CLI is `streaq run src.mus.core.streaq_broker:worker`.
- Root `.env` is gitignored and stores `SECRET_KEY`.
