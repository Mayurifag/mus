# Implement Event-Driven File Watcher Pipeline

## User Problem
The application's music library is static after the initial startup scan. It does not reflect real-time changes made to the music files on the disk, such as additions, modifications, moves, or deletions. This makes the library stale and requires a manual application restart to see changes.

## High-Level Solution
Implement a `watchdog`-based file system watcher that runs as a background service. This service will monitor the music directory for all file events and enqueue corresponding jobs to `RQ` for asynchronous processing. A new worker function, `process_track_enhancements`, will handle CPU-intensive tasks like accurate duration analysis (using FFprobe) and cover art extraction. Other workers will handle file deletion and moves. This ensures the database and UI are kept in sync with the filesystem in near real-time without blocking the main application.

## Success Metrics
- New audio files added to the music directory appear in the library automatically with correct metadata, duration, and cover art.
- Changes to a file's metadata (e.g., via an external tag editor) are reflected in the library after a short processing delay.
- Tracks deleted from the file system are removed from the library, and their associated cover art is also deleted.
- Tracks moved within the music directory have their file paths updated in the database without losing their identity or metadata.
- All file processing happens in background workers, not blocking the main FastAPI application.
- The UI is updated via existing SSE mechanisms after a file is processed.
- The implementation passes all `make ci` checks.

## Detailed Description
This feature introduces a real-time file monitoring system to replace the one-time initial scan for subsequent library updates. It's built on `watchdog` and `RQ`.

1.  **File Watcher Service (`watchdog`)**: A `FileWatcherService` will be created in a new `src/mus/service/file_watcher_service.py` file. It will contain a `MusicFileEventHandler` that inherits from `watchdog.events.FileSystemEventHandler`. This handler will be responsible for catching `on_created`, `on_modified`, `on_deleted`, and `on_moved` events and enqueuing corresponding jobs to RQ. The service will be started in the application's `lifespan` in `src/mus/main.py`.

2.  **Worker Logic**: The existing `process_slow_metadata` worker will be replaced and expanded:
    *   **Enhancements Worker (`process_track_enhancements`)**: This single worker will be responsible for all slow, post-scan processing. It will first call a new `ffprobe` utility to get the accurate duration and update the track record. Then, it will call the existing `CoverProcessor` to extract and save cover art. Finally, it will mark the track's status as `COMPLETE` and broadcast a UI update.
    *   **Deletion Worker (`process_file_deletion`)**: A new worker to handle the removal of a track record and its associated cover art files.
    *   **Move Worker (`process_file_move`)**: A new worker to update a track's file path.

3.  **Event Handling**:
    *   **Create/Modify**: The `on_created` and `on_modified` handlers will perform a quick `mutagen` metadata extraction and upsert, then enqueue the `process_track_enhancements` job. The `on_created` handler will check for an existing `inode` to robustly handle files that were moved while the application was offline, preventing duplicates.
    *   **Delete/Move**: The `on_deleted` and `on_moved` handlers will enqueue their respective simple, fast-executing jobs.

4.  **Queues**: The `low_priority` queue will handle the `process_track_enhancements` jobs. A `high_priority` queue will be used for `delete` and `move` jobs to ensure they are processed quickly.

5.  **Consistency**: The initial startup scanner will be updated to use the same `process_track_enhancements` worker, ensuring consistent processing logic for both initial and subsequent file additions.

## Subtasks

### [ ] 1. Implement Full File Watcher Pipeline
**Description**: Create the file watcher, event handlers, and all associated RQ workers for real-time library synchronization in a single, comprehensive step.
**Details**:
- **Part 1: FFprobe Utility**
  - Create a new file `src/mus/util/ffprobe_analyzer.py`.
  - In it, implement a function `get_accurate_duration(file_path: Path) -> int`. This function should use `ffmpeg.probe(str(file_path))`, extract the duration from `format.duration`, cast it to an integer, and return it. It must handle any exceptions gracefully by logging a warning and returning `0`.
- **Part 2: Worker Tasks Refactoring**
  - In `src/mus/service/worker_tasks.py`:
    - Rename `process_slow_metadata` to `process_track_enhancements(track_id: int)`.
    - Implement its logic: it must get a DB session, fetch the `Track`, call `get_accurate_duration` and update `track.duration`, call `CoverProcessor.extract_cover_from_file` and `CoverProcessor.process_and_save_cover`, update `track.has_cover` if successful, set `track.processing_status` to `COMPLETE`, commit changes, and finally call `broadcast_sse_event(action_key="reload_tracks")`.
    - Create a new worker `def process_file_deletion(file_path_str: str):`. It must find the track by path, delete its associated original and small cover files from `settings.COVERS_DIR_PATH`, delete the track record from the database, and then broadcast a `reload_tracks` SSE event.
    - Create a new worker `def process_file_move(src_path: str, dest_path: str):`. It must find the track by `src_path` and update its `file_path` to `dest_path`.
- **Part 3: File Watcher Service**
  - Create a new file `src/mus/service/file_watcher_service.py`.
  - Define a `MusicFileEventHandler(FileSystemEventHandler)` class.
    - `on_created` and `on_modified` methods: These should extract fast metadata with `mutagen`, `upsert` the track data with `processing_status=METADATA_DONE`, and then enqueue a job for `process_track_enhancements` to the `low_priority` queue. The `on_created` handler must first check for an existing track with the same `inode` to handle offline moves correctly.
    - `on_deleted`: Enqueue `process_file_deletion` to the `high_priority` queue.
    - `on_moved`: Enqueue `process_file_move` to the `high_priority` queue.
  - Define a `FileWatcherService` class to manage the `watchdog.observers.Observer`, with `start()` and `stop()` methods.
- **Part 4: Integration**
  - Modify `src/mus/main.py`: In the `lifespan` manager, instantiate and start the `FileWatcherService`. Ensure the observer is stopped gracefully on application shutdown. You will need to define a `high_priority` queue here.
  - Modify `src/mus/service/scanner_service.py`: Update `WORKER_FUNCTION_NAME` to `"src.mus.service.worker_tasks.process_track_enhancements"`.
- **Part 5: Repository Enhancement**
  - In `src/mus/infrastructure/persistence/sqlite_track_repository.py`, add a new method `get_by_inode(self, inode: int) -> Optional[Track]` and `delete_by_path(self, file_path: str) -> bool`.
**Filepaths to Modify**: `src/mus/main.py,src/mus/service/worker_tasks.py,src/mus/service/scanner_service.py,src/mus/infrastructure/persistence/sqlite_track_repository.py`
**Filepaths to Create**: `src/mus/util/ffprobe_analyzer.py,src/mus/service/file_watcher_service.py`
**Relevant Make Commands**: `make back-test, make back-lint`