# Implement Initial Library Scan on Startup

## User Problem
The application does not automatically scan the music library on startup. This leaves the library empty until files are processed by a file watcher, which is not yet fully implemented for all event types. A comprehensive initial scan is needed to populate the database when the application starts.

## High-Level Solution
Implement a service that runs as a background task on application startup. This service will perform a full scan of the music directory. For each audio file, it will synchronously extract basic metadata (`mutagen`) and file `inode` in batches, `upsert` this information into the `Track` table, and then enqueue a job to a `low_priority` RQ queue for slower, more intensive processing like accurate duration analysis and cover art extraction.

## Success Metrics
- On application startup, a background task for the initial scan is launched and logged.
- All valid audio files in the `MUSIC_DIR_PATH` are processed by the scanner.
- Each processed file results in a `Track` record in the database with `processing_status` set to `METADATA_DONE`.
- For each track processed, a job is successfully enqueued to the `low_priority` queue pointing to a placeholder worker function.
- The main FastAPI application starts up and becomes responsive without waiting for the full scan to complete.
- The implementation passes all `make ci` checks.

## Detailed Description
This task creates the initial scanning mechanism that populates the music library from the filesystem.

1.  **Metadata Extraction Utility**: A helper function `extract_fast_metadata` will be created in a new file, `src/mus/util/metadata_extractor.py`. This function will take a file path and use `mutagen` to extract `title` and `artist`. It will also get the file's `inode` number using `os.stat`. If tags are missing, the filename (without extension) will be used as the title.

2.  **Worker Task Placeholder**: A new file `src/mus/service/worker_tasks.py` will be created. It will contain a placeholder function `def process_slow_metadata(track_id: int): pass`. This function will serve as the target for the RQ jobs and will be implemented in a subsequent task.

3.  **Scanner Service**: A new `InitialScanner` class will be created in a new file, `src/mus/service/scanner_service.py`. This service will orchestrate the scan.
    -   It will use the existing `FileSystemScanner` to find all audio files.
    -   It will process files in batches (e.g., size of 100) to manage memory.
    -   For each file, it will call the metadata utility.
    -   It will create `Track` model instances, setting `processing_status` to `METADATA_DONE` and `duration` to a default of `0`.
    -   After each batch, it will loop through the `Track` objects and use `track_repository.upsert_track()` to save them to the database.
    -   For each successfully upserted track, it will enqueue a job to the `low_priority` queue, calling the placeholder worker function with the `track.id`.

4.  **Startup Integration**: The `lifespan` async context manager in `src/mus/main.py` will be modified to:
    -   Establish a connection to Redis based on `settings.DRAGONFLY_URL`.
    -   Create an `rq.Queue` instance for the `low_priority` queue.
    -   Instantiate the `InitialScanner` with the repository and queue.
    -   Launch the `scanner.scan()` method as a non-blocking background task using `asyncio.create_task()`.

## Implementation Considerations
- The `rq` and `redis` libraries are already in `pyproject.toml`.
- All new modules (`service`, `util`) will require `__init__.py` files to be proper packages.
- The worker function must be defined at the top level of its module for RQ to import it correctly.

## Subtasks

### [x] 1. Implement Initial Scan Service and Startup Integration
**Description**: Create the `InitialScanner` service, a metadata extraction utility, a placeholder for worker tasks, and integrate the scan into the application's startup lifecycle.
**Details**:
-   Create a new file `src/mus/util/metadata_extractor.py`.
    -   Implement a function `extract_fast_metadata(file_path: Path) -> dict | None` that uses `mutagen` to get `title` and `artist`, and `os.stat` to get `inode`. Use the filename if tags are absent. `duration` should be `0` initially.
-   Create a new file `src/mus/service/worker_tasks.py`.
    -   Define a placeholder function `def process_slow_metadata(track_id: int): pass`.
-   Create a new file `src/mus/service/scanner_service.py`.
    -   Define the `InitialScanner` class. Its `__init__` should accept a `track_repository` and a `low_priority_queue`.
    -   Implement an `async def scan(self)` method that finds all audio files, processes them in batches, calls `extract_fast_metadata`, upserts `Track` objects to the DB with `processing_status` set to `METADATA_DONE`, and enqueues jobs to `low_priority_queue` with the `track.id`.
-   Create `__init__.py` files in `src/mus/service/` and `src/mus/util/`.
-   Modify `src/mus/main.py`:
    -   In the `lifespan` context manager, import `asyncio`, `Redis` from `redis.asyncio`, `Queue` from `rq`.
    -   Instantiate the Redis client and the `low_priority_queue`.
    -   Get a DB session to create a `SQLiteTrackRepository`.
    -   Instantiate `InitialScanner` with the repository and queue.
    -   Define an `async def run_initial_scan()` wrapper function.
    -   Use `asyncio.create_task(run_initial_scan())` to run the scan in the background after the DB and directories are confirmed to exist.
**Filepaths to Modify**: `src/mus/main.py`
**Filepaths to Create**: `src/mus/service/__init__.py,src/mus/service/scanner_service.py,src/mus/service/worker_tasks.py,src/mus/util/__init__.py,src/mus/util/metadata_extractor.py`
