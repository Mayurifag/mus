# Implement Track Downloader from URL using yt-dlp

## User Problem
Users want a seamless way to add new music to their library from various online sources (like YouTube, SoundCloud, etc.) by simply providing a URL. They need control over the final metadata, such as the track title and artist, before the file is permanently added to their collection. The process should be interactive, providing clear feedback on its status and allowing for user review.

## High-Level Solution
Implement a new feature that allows users to initiate a download from a URL via a dedicated UI. This action will trigger a non-blocking backend process using `yt-dlp` to download the audio file into a temporary, isolated directory. Once complete, the backend will notify the frontend via SSE, presenting a confirmation modal pre-filled with extracted metadata. The user can review, edit, and confirm these details. Upon confirmation, a final backend process will apply the user-approved metadata, rename the file, and move it into the main music library, where the existing file-watcher will integrate it.

## Success Metrics
- A user can paste a supported URL, initiate a download, and see progress via UI feedback.
- After download, the user is presented with a modal to confirm or edit the track's title and artist.
- After confirmation, the new track appears in their main library with the correct metadata and cover art.
- The process is non-blocking, and the user can continue using the application while a download is in progress.
- The system correctly handles download failures and provides feedback to the user.
- All new code is covered by unit and integration tests, and `make ci` passes.

## Detailed Description
The feature will be implemented in several stages, starting with the backend infrastructure, followed by the core download logic, and finally the frontend UI components.

**Backend Architecture:**
- A new API endpoint `/api/v1/downloads/url` will accept a URL and enqueue a background job.
- A Redis lock will prevent concurrent downloads.
- A `streaq` worker will execute the download using `yt-dlp` in a secure subprocess, saving the file to a temporary directory (`/app_data/tmp/downloads/<uuid>/`).
- SSE events (`download_started`, `download_failed`, `download_ready_for_review`) will communicate the status to the frontend.
- Upon success, the job will extract metadata and cover art and include them in the `download_ready_for_review` event payload.
- A second API endpoint, `/api/v1/downloads/confirm`, will finalize the process by applying user-edited metadata, renaming the file, and moving it to the main music library.

**Frontend Architecture:**
- A new `DownloadManager` component in the right sidebar will contain the URL input field and display download status.
- A `downloadStore` will manage the state of the download process (`idle`, `downloading`, `awaiting_confirmation`, `failed`).
- The existing `TrackMetadataModal` will be reused and extended to handle the confirmation step.
- The main layout will listen for SSE events to update the `downloadStore` and trigger the confirmation modal.

**Security & Configuration:**
- The `yt-dlp` command will be executed via `asyncio.create_subprocess_exec` to prevent command injection.
- All new endpoints will be protected by the existing authentication mechanism.
- Users can provide a `cookies.txt` file by mounting a volume to `/app_data/yt-dlp-config/` for downloads requiring authentication.

## Dependencies
- Add `yt-dlp` to `backend/pyproject.toml`.
- Ensure `ffmpeg` is available in both development and production Docker environments (it already is).

## Subtasks

### [ ] 1. Backend: Implement Download Initiation API and Job
**Description**: Create the API endpoint to start a download from a URL and set up the corresponding background job structure.
**Details**:
- Add `yt-dlp>=2023.12.30` to `backend/pyproject.toml` dependencies.
- Create a new router in a new file `backend/src/mus/infrastructure/api/routers/download_router.py`.
- Implement a `POST /api/v1/downloads/url` endpoint that accepts a JSON body with a `url: str` field.
- The endpoint must use Redis to set a global lock (key `download_lock:global`) with a 10-minute expiry. If the lock is already present, return a `429 Too Busy` status.
- If the lock is acquired, enqueue a new `streaq` job named `download_track_from_url` with the URL as an argument. Return a `202 Accepted` status.
- Create the job file `backend/src/mus/infrastructure/jobs/download_jobs.py` and define the `download_track_from_url` task. For this subtask, the job should only log its execution and release the Redis lock. It should not perform the download yet.
- Register the new router in `backend/src/mus/main.py`.
- Register the new job module in `backend/src/mus/core/streaq_broker.py`.
- Write tests in a new file `backend/tests/api/test_download_router.py` to verify: a successful request returns 202 and enqueues a job; a second request returns 429 when the lock is active; the Redis lock is correctly set and released by the placeholder job.
**Filepaths to Modify**: `backend/pyproject.toml,backend/src/mus/main.py,backend/src/mus/core/streaq_broker.py,backend/src/mus/infrastructure/api/routers/download_router.py,backend/src/mus/infrastructure/jobs/download_jobs.py,backend/tests/api/test_download_router.py`
**Relevant Make Commands (Optional)**: `make back-test`