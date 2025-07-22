# Reimplement Backend Event and File Processing System

## User Problem
The current backend file processing system, which relies on RQ and Watchdog, needs to be replaced with a more modern, robust, and performant architecture. The new system must handle initial library scans efficiently, process file system events in real-time, provide better error handling, and be architecturally clean.

## High-Level Solution
The entire event-driven file processing pipeline will be rebuilt from scratch. We will replace `rq` with `arq` for asynchronous task queuing and `watchdog` with `watchfiles` for file system monitoring. The new architecture will follow clean architecture principles, separating application use cases from infrastructure adapters (file watching, job queuing). The initial scan will be a two-phase process: a rapid, parallelized fast metadata scan, followed by a batched slow metadata analysis (covers, duration, tag standardization) that reuses job logic but runs directly without the queue. A new error management feature will be added, allowing users to view and act upon failed processing tasks via the frontend. The `Track` database model will be updated to store detailed information about the last processing error.

**For additional context on the intended event flow and state transitions, refer to `backend/STATE_MACHINE.md`.**

## Success Metrics
- The application successfully uses `arq` and `watchfiles`, and the old `rq` and `watchdog` dependencies are removed.
- The initial library scan is parallelized, processes files sorted by modification time descending, and completes both fast and slow metadata processing without using the `arq` queue for the slow part.
- The tag standardization process during the initial scan (and in `arq` jobs) preserves the original file modification time and does not trigger redundant file watcher events.
- Real-time file system events (create, modify, delete, move) are correctly handled and trigger the appropriate background jobs.
- Failed processing jobs are recorded in the database, and the error is cleared upon successful reprocessing.
- A new "Errors" section appears in the frontend sidebar, listing failed tracks and allowing users to re-queue jobs or delete the track.
- `make ci` passes with zero errors.

## Detailed Description
The implementation will be a single, large-scale refactoring of the backend's file processing logic.
1.  **Dependency and Model Changes**: `pyproject.toml` will be updated to replace `rq` and `watchdog` with `arq` and `watchfiles`. The `Track` entity will be modified to include a `last_error` JSON field to store a structured error object (`timestamp`, `job_name`, `error_message`).
2.  **New Architecture**:
    -   `core`: Contains the shared `arq` pool setup (`core/arq_pool.py`).
    -   `application/use_cases`: A new `InitialScanUseCase` will orchestrate the startup scan.
    -   `infrastructure/file_watcher`: A new `watcher.py` will act as an adapter for `watchfiles`.
    -   `infrastructure/jobs`: This directory will contain all `arq` job definitions (`metadata_jobs.py`, `file_system_jobs.py`) and the worker configuration (`worker.py`).
3.  **Initial Scan Logic**: The `InitialScanUseCase` will execute in two distinct phases:
    -   **Phase 1 (Fast Scan)**: Find all audio files, sort them descending by modification time, and use a `ProcessPoolExecutor` for parallel fast metadata extraction. Results are bulk-upserted to the DB.
    -   **Phase 2 (Slow Scan)**: After the fast scan, it will query for all pending tracks and process them in batches. This phase will directly call a reusable slow metadata processing function (shared with `arq` jobs) to handle accurate duration, cover art, and tag standardization. This phase does **not** use the `arq` queue.
4.  **Tag Standardization & `mtime` Preservation**: The reusable slow metadata function must implement a mechanism to preserve file integrity. Before saving standardized ID3v2.3 tags, it will record the file's current `mtime`. After saving, it will use `os.utime` to restore the original `mtime`. A short-lived Redis lock will be set before the file write to signal the file watcher to ignore this specific modification event.
5.  **Error Handling UI**:
    -   A new `errors_router.py` will expose `GET /api/v1/errors/tracks` to list failed jobs and `POST /api/v1/errors/tracks/{track_id}/requeue` to retry a failed job by reading the job name from the track's `last_error` field.
    -   The frontend will use the existing `DELETE /api/v1/tracks/{track_id}` for deletion.

## Subtasks

### [ ] 1. Overhaul Backend for `arq` and `watchfiles` with Error Handling UI
**Description**: A comprehensive, single-subtask reimplementation of the entire backend file processing and event system, including database, architecture, jobs, and the corresponding frontend error management UI.
**Details**:

**Part A: Foundation and Dependencies**
1.  In `backend/pyproject.toml`, remove `rq` and `watchdog`. Add `arq` and `watchfiles`. Run `make back-lock` followed by `make back-sync`.
2.  Update `backend/src/mus/domain/entities/track.py`:
    -   Import `JSON` from `sqlalchemy`.
    -   Replace `last_error_message: Optional[str] = Field(default=None)` with `last_error: Optional[Dict[str, Any]] = Field(default=None, sa_column=Column(JSON))`.
3.  Delete the following obsolete files and directories: `backend/src/mus/util/redis_utils.py`, `backend/src/mus/util/queue_utils.py`, `backend/src/mus/service/`.

**Part B: `arq` Worker and Job Setup**
1.  Create `backend/src/mus/core/arq_pool.py`. Define a function `get_arq_pool()` that returns an `ArqRedis` instance using `RedisSettings`.
2.  Create the `backend/src/mus/infrastructure/jobs/` directory.
3.  Create a new file `backend/src/mus/application/use_cases/process_track_metadata.py` to house the reusable slow metadata logic.
    -   Define an `async` function `process_slow_metadata_for_track(track_id: int)`. This function will be the core logic called by both the initial scan and the `arq` job.
    -   Inside this function, implement the logic for accurate duration, cover art, and ID3v2.3 tag standardization.
    -   **Crucially**, for tag standardization: before saving the file, get its `mtime` and set a Redis lock (e.g., `app_write_lock:<filepath>` with a 5s TTL). After saving, use `os.utime` to restore the original `mtime`.
4.  Create `backend/src/mus/infrastructure/jobs/metadata_jobs.py`:
    -   Define an `async` job `process_slow_metadata(ctx, track_id: int)`.
    -   This job will be a thin wrapper around a `try...except` block that calls `process_slow_metadata_for_track`. On exception, it updates the track's `last_error` field and broadcasts a `track_error` SSE. On success, it ensures the `last_error` field is `NULL` and broadcasts `track_updated`.
5.  Create `backend/src/mus/infrastructure/jobs/file_system_jobs.py`:
    -   Define jobs: `handle_file_created`, `handle_file_modified`, `handle_file_deleted`, `handle_file_moved`, and `delete_track_with_files(ctx, track_id: int)`.
    -   The `handle_file_modified` job **must** check for the Redis `app_write_lock` for the file path. If the lock exists, the job should immediately return to prevent processing events triggered by the application itself (e.g., tag standardization).
6.  Create `backend/src/mus/infrastructure/jobs/worker.py` to define the `WorkerSettings` for `arq`, listing all job functions and queue priorities.

**Part C: Application Use Cases and Infrastructure Adapters**
1.  Create `backend/src/mus/application/use_cases/initial_scan.py`:
    -   Implement the `InitialScanUseCase` class. Its `execute` method will perform the two-phase scan:
        -   **Phase 1**: Parallel fast metadata extraction using `ProcessPoolExecutor`, sorting by `mtime` descending, and a final bulk upsert.
        -   **Phase 2**: After Phase 1, query all pending tracks from the DB. Process them in batches, calling the reusable `process_slow_metadata_for_track` function for each track. This phase must run directly, not through the `arq` queue.
2.  Create `backend/src/mus/infrastructure/file_watcher/watcher.py`:
    -   Implement `async def watch_music_directory` using `watchfiles.awatch` to enqueue jobs from `file_system_jobs.py`.

**Part D: API and Application Integration**
1.  Update `backend/src/mus/main.py`: In the `lifespan` manager, execute `InitialScanUseCase` and start the `watch_music_directory` task.
2.  Update `backend/src/mus/infrastructure/api/routers/track_router.py`: Modify the `DELETE /{track_id}` endpoint to enqueue `infrastructure.jobs.file_system_jobs.delete_track_with_files`.
3.  Create `backend/src/mus/infrastructure/api/routers/errors_router.py`:
    -   Implement `GET /tracks`.
    -   Implement `POST /tracks/{track_id}/requeue`: This endpoint will fetch the track, read the `job_name` from its `last_error` field, clear the `last_error` field, and re-enqueue the job from that name.
4.  Add the new `errors_router` to the FastAPI app in `backend/src/mus/main.py`.

**Part E: Docker and Configuration**
1.  Modify `docker/production/supervisord.conf`, `docker/docker-compose.override.yml.example`, and your local `docker-compose.override.yml` to replace the `rq worker` command with `arq src.mus.infrastructure.jobs.worker.WorkerSettings`.

**Part F: Frontend Error UI**
1.  In `frontend/src/lib/services/apiClient.ts`, add `fetchErroredTracks` and `requeueTrack`. Ensure `deleteTrack` is used for deletion.
2.  Create `frontend/src/lib/components/domain/ErrorItem.svelte`. It will display error info and provide "Re-queue" and "Delete" buttons. The delete button will call the existing `deleteTrack` function.
3.  Modify `frontend/src/lib/components/layout/RightSidebar.svelte`: Fetch and display errored tracks, conditionally rendering the "Errors" section. Handle SSE events to keep the list updated dynamically.

**Filepaths to Modify**:
`backend/pyproject.toml`, `backend/requirements.txt`, `backend/src/mus/domain/entities/track.py`, `backend/src/mus/main.py`, `backend/src/mus/infrastructure/api/routers/track_router.py`, `docker/production/supervisord.conf`, `docker/docker-compose.override.yml.example`, `frontend/src/lib/services/apiClient.ts`, `frontend/src/lib/components/layout/RightSidebar.svelte`

**Filepaths to Create**:
`backend/src/mus/core/arq_pool.py`, `backend/src/mus/application/use_cases/initial_scan.py`, `backend/src/mus/application/use_cases/process_track_metadata.py`, `backend/src/mus/infrastructure/jobs/metadata_jobs.py`, `backend/src/mus/infrastructure/jobs/file_system_jobs.py`, `backend/src/mus/infrastructure/jobs/worker.py`, `backend/src/mus/infrastructure/file_watcher/watcher.py`, `backend/src/mus/infrastructure/api/routers/errors_router.py`, `frontend/src/lib/components/domain/ErrorItem.svelte`

**Filepaths to Delete**:
`backend/src/mus/util/redis_utils.py`, `backend/src/mus/util/queue_utils.py`, `backend/src/mus/service/` (entire directory)

**Relevant Make Commands**: `make back-install`, `make ci`