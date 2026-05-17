# Agent Notes

- Always use `make` targets. Do not run `cargo`, `vitest`, `docker compose`, or `npm` directly.
- Main targets: `make up`, `make down`, `make logs [backend|frontend]`, `make ci`, `make back-ci`, `make back-test`, `make front-test`, `make back-lint`, `make front-lint`, `make back-sh`, `make e2e`, `make ps`.
- Production verification targets: `make prod-image`, `make prod-smoke`, `make e2e-current-image`, `make prod-security-scan`, `make prod-verify`.
- Use `make rebuild-backend-image` to refresh baked backend tools like yt-dlp without deleting volumes.
- Use `make rebuild-frontend-image` to refresh baked frontend tools like npm.
- Use `make prod-image` to rebuild the production image locally.
- Use `make prod-image-fresh` only when Docker cache must be bypassed.
- Use `make e2e-current-image` after `make prod-image` when the image is already built and the e2e rebuild would only re-fetch registry metadata.
- `docker/docker-compose.override.yml` is gitignored and required for local dev. Create it from `docker/docker-compose.override.yml.example` when missing.
- Production deployments run the app behind external authentication; do not add app-level auth unless explicitly requested.
- Database and generated covers are derived container data; do not add Docker volumes for them unless explicitly requested.
- Local music mount is `/Volumes/sdcard-apfs/OpenCloud/Personal/Music:/app_data/music`.
- Backend is Rust in `backend-rs`.
- Keep Markdown docs and roadmap notes current when changes affect project behavior or durable guidance.
