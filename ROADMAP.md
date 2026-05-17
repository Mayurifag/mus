# Roadmap

Primary goal: keep mus as a small, single-container music player/downloader while the Rust backend becomes the only backend and production stays easy to reason about.

## Current Baseline

- Rust backend is the active backend.
- Old Python backend, Redis, streaq, Python worker, supervisord, cron, nginx runtime, and Node runtime are removed from the active production path.
- Production target is one long-lived process: `mus-backend`.
- SvelteKit uses static output and Rust serves the built frontend.
- `make ci` and `make prod-image` pass on the Rust/static production image.
- Browser e2e passed against the freshly built `mus:latest` image tagged as `mus:e2e-test`; the `make e2e` wrapper hit a transient Docker Hub metadata EOF while rebuilding.
- Production smoke passed for healthcheck, frontend root, SPA fallback, and `/api/v1/tracks`.
- Rust has API contract tests outside `main.rs` in `backend-rs/tests/api_contract.rs`.
- Rust quality tooling exists in `make back-ci`: fmt check, check, clippy with warnings denied, tests, machete, and audit.

## Migration Finish Line

### 1. Verify Rust ID3 Tagging

The old Python app is gone. MP3/WAV/AIFF ID3 tagging now uses Rust `id3`; other media rewrites still use `ffmpeg`.

- Keep MP3 ID3v2.3/UTF-8 and embedded-cover regression coverage.
- Keep WAV ID3v2.4-to-ID3v2.3 regression coverage.
- Add broader real-file coverage for FLAC/WAV files with covers and files inside folders.
- Keep `yt-dlp`, `ffmpeg`, and `ffprobe`; they are on-demand media tools and `yt-dlp` is intentionally retained for ready-to-use YouTube and provider downloads.

### 2. Improve Contract Confidence

- Expand contract tests from route coverage to behavior coverage.
- Cover success and failure cases for external-tool routes without real network calls.
- Add stream and cover assertions for `ETag`, `Cache-Control`, `Accept-Ranges`, `Content-Range`, content type, and missing-file behavior.
- Add validation/error-shape tests for malformed JSON, missing fields, unsupported file types, duplicate filenames, and permission failures.
- Keep FastAPI-compatible error shape: `{"detail": ...}`.
- Add e2e scenarios for duration persistence, covered FLAC/WAV files, files inside folders, and external delete/create changes.
- Add e2e to deployment CI once runtime behavior is stable enough.

### 3. Startup And File Processing Robustness

- Before write checks or scans, handle a missing/unmounted music folder cleanly.
- Preserve read-only-folder behavior: disable editing/downloading without breaking startup.
- Detect destination filename collisions on edit/upload/download and block with a clear UI/server error.
- Improve external file change handling for delete-and-create, moved files, and metadata-only updates.
- Rename noisy or misleading events like `track_updated` from background metadata work if the frontend should not show a user-facing notification.
- Standardize tags and write duration/metadata with the fewest possible file rewrites.
- Keep startup scan non-blocking enough that health checks stay useful.

### 4. Dependency And Image Maintenance

- Update Rust crates through make-backed flows and verify with `make back-ci`.
- Update frontend/e2e npm deps through make-backed flows, not ad hoc package manager commands.
- Svelte can use Vite 8 via `@sveltejs/vite-plugin-svelte@7.1.2`; do not assume Vite 8 is blocked.
- Remove unused adapters or packages after static production is fully settled.
- Re-measure production memory and image size after the Rust ID3 media change.

## Product Backlog

### Player And UI

- Fix any remaining restore-on-open-tab bugs; consider local storage plus periodic server sync if it simplifies state restore.
- Improve desktop player footer layout so controls do not steal too much artist/title space.
- Finish slider styling: smaller by default, larger on hover, less saturated while idle, blue on hover/play.
- Add hotkeys for player controls and show hotkey hints on hover.
- Add marquee/overflow behavior for long text where truncation hurts usability.
- Render a play button under album cover in the track list.
- Make shuffled history tracks playable on click.
- Consider a `Play next` action.

### Library Management

- Add sorting by different track fields and directions.
- Add fast search that can later search local library, YouTube, VK, and other sources.
- Add cover-art editing by fetching candidates from YouTube/search/providers; avoid upload handling until it is clearly needed.
- Add optional track download/export from the edit window if there is a real use case.
- Define Artist, Album, and Playlist entities later.
- Support many-to-many track/artist/album/playlist relationships.
- Parse multiple artists into unique artist records and keep aliases such as `Тату` / `t.A.T.u.`.

### Download Sources

- Add startup or scheduled `yt-dlp` update only if the manual endpoint is not enough.
- Add VK search/download support.
- Consider zaycev.net if it remains useful.

### Operations

- Decide whether separate database/cover/music Docker volumes are still useful or whether one data volume is simpler.
- Improve app-wide logging only where it helps diagnose real production issues.
- Consider startup user/group ownership alignment for mounted music folders.
- Log slow e2e steps if e2e time becomes a recurring problem.

## Probably Not Needed Now

- `cargo nextest`: not needed while Rust tests are small and `cargo test` is fast enough.
- `cargo llvm-cov`: useful later if contract coverage becomes hard to reason about, not necessary now.
- More Rust architecture layers: avoid until repeated code forces it.
- Replacing `ffmpeg`/`ffprobe`: not worth it; they solve real media edge cases and run on demand.
- Redis/pub-sub: do not reintroduce unless SSE or downloads need cross-process fanout again.
- git-crypt for plain examples: only use git-crypt if example/config files must contain encrypted sensitive values. Normal `.example` files should stay plaintext.

## Cleanup Rules

- Delete legacy code only after a passing contract/e2e/prod-smoke checkpoint.
- Keep make target names stable where useful, but make implementations Rust-first.
- Do not add new tooling unless it catches real bugs or removes maintenance burden.
- Keep `AGENTS.md` small; add only durable project-specific rules.
