# Agent Notes

- Always use `make` targets. Do not run `cargo`, `vitest`, `docker compose`, or `npm` directly.
- Main targets: `make up`, `make down`, `make logs [backend|frontend]`, `make ci`, `make back-ci`, `make back-test`, `make front-test`, `make back-lint`, `make front-lint`, `make back-sh`, `make e2e`, `make e2e-current-image`, `make ps`.
- Use `make rebuild-backend-image` to refresh baked backend tools like yt-dlp without deleting volumes. Use `make prod-image` to rebuild the production image locally.
- Use `make e2e-current-image` after `make prod-image` when the image is already built and the e2e rebuild would only re-fetch registry metadata.
- `docker/docker-compose.override.yml` is gitignored and required for local dev. Create it from `docker/docker-compose.override.yml.example` when missing.
- Local music mount is `/Volumes/sdcard-apfs/OpenCloud/Personal/Music:/app_data/music`.
- Backend is Rust in `backend-rs`.
