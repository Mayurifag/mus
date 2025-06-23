# Implement Track History and Queue Monitoring

## User Problem
There is no way to track or revert changes to a track's metadata if an external tool modifies the file incorrectly. Additionally, there is no visibility into the health of the background job processing system, making it hard to diagnose issues with file watching or metadata processing.

## High-Level Solution
First, introduce a `TrackHistory` table to log the last 5 metadata states (`title`, `artist`, `duration`) for each track whenever a change is detected. This will be coupled with new API endpoints to view a track's history and to roll back to a specific historical version.
Second, implement a new monitoring API endpoint to expose the current depth (number of jobs) of the `high_priority` and `low_priority` RQ queues.

## Success Metrics
- Metadata changes from file modifications are saved to the `TrackHistory` table.
- The history for any given track is automatically pruned to the last 5 entries.
- A `POST` request to the rollback endpoint successfully restores the track's metadata in the `Track` table.
- A `GET` request to the history endpoint returns the list of historical versions for a track.
- A `GET` request to the queue monitoring endpoint returns the current job counts for all defined queues.
- All new code is tested and `make ci` passes without any errors or warnings.

## Detailed Description
This task involves two separate but related enhancements to the application's robustness and observability.

**1. Track History and Rollback:**
A new `TrackHistory` entity and corresponding database table will be created to store snapshots of a track's metadata before it is updated. The fields to be versioned are `title`, `artist`, and `duration`.

-   **Data Model**: A `TrackHistory` model will be created with a foreign key to the `Track` model. It will store the previous values of the track's metadata and a timestamp of when the change occurred.
-   **Logic**: The change detection and history creation logic will be integrated into the `process_file_upsert` worker task in `src/mus/service/worker_tasks.py`. Before upserting a track, the existing record will be fetched. After the upsert, the old and new metadata will be compared. If a change is detected, a new `TrackHistory` record will be created from the old data.
-   **History Pruning**: The application logic will ensure that only the last 5 history entries per track are retained. When a new history record is added, if the count for that track exceeds 5, the oldest entry will be deleted.
-   **API**:
    -   A new router will be created for history-related endpoints.
    -   `GET /api/v1/tracks/{track_id}/history`: Fetches all history entries for a given track.
    -   `POST /api/v1/tracks/history/{history_id}/rollback`: Takes a `history_id`, retrieves that historical record, and updates the main `Track` record with its data. This action will not create a new history entry itself.

**2. Queue Monitoring:**
A simple monitoring endpoint will be added to provide visibility into the RQ job queues.

-   **API**:
    -   A new router will be created for monitoring endpoints.
    -   `GET /api/v1/monitoring/queues`: This endpoint will return a list of objects, each containing a queue's name and its current job count.
-   **Logic**: The endpoint will use the `rq` library's utilities to connect to the queues (`high_priority`, `low_priority`) and retrieve their job counts.

## Subtasks

### [ ] 1. Implement Track History, Rollback API, and Queue Monitoring
**Description**: Implement the full track history and queue monitoring features in a single, comprehensive step.
**Details**:
-   **Part 1: TrackHistory Model and DTO**
    -   Create `src/mus/domain/entities/track_history.py`. Define a `TrackHistory` `SQLModel` with fields: `id`, `track_id` (foreign key to `track.id`), `title`, `artist`, `duration`, and `changed_at` (integer timestamp).
    -   Create `src/mus/application/dtos/track_history.py`. Define a `TrackHistoryDTO` pydantic model based on the `TrackHistory` entity.
-   **Part 2: TrackHistory Repository**
    -   Create `src/mus/infrastructure/persistence/sqlite_track_history_repository.py`.
    -   Implement `SQLiteTrackHistoryRepository` with methods:
        -   `add(self, history_entry: TrackHistory) -> TrackHistory`: Adds a new history record.
        -   `get_by_track_id(self, track_id: int) -> List[TrackHistory]`: Retrieves all history for a track, ordered by `changed_at` descending.
        -   `get_by_id(self, history_id: int) -> Optional[TrackHistory]`: Retrieves a single history entry by its ID.
        -   `prune_history(self, track_id: int, keep: int = 5)`: Deletes the oldest history entries for a track, keeping the most recent `keep` entries.
-   **Part 3: Integrate History Creation into Worker**
    -   Modify `process_file_upsert` in `src/mus/service/worker_tasks.py`.
    -   Before calling `upsert_track`, fetch the existing track by its path using `get_track_by_path`.
    -   After `upsert_track`, if an `existing_track` was found, compare its `title`, `artist`, and `duration` with the newly upserted track's data.
    -   If there's a difference, create a `TrackHistory` instance from the `existing_track`'s data, add it to the database, and then prune the history for that track to the last 5 entries. You'll need to create corresponding utility functions in `src/mus/util/db_utils.py` for history operations.
-   **Part 4: History and Rollback API**
    -   Create `src/mus/infrastructure/api/routers/history_router.py`.
    -   Implement `GET /api/v1/tracks/{track_id}/history` which returns a `List[TrackHistoryDTO]`.
    -   Implement `POST /api/v1/tracks/history/{history_id}/rollback`. This endpoint will:
        -   Fetch the `TrackHistory` entry by `history_id`.
        -   Fetch the corresponding `Track`.
        -   Update the `Track`'s `title`, `artist`, and `duration` from the history entry.
        -   Save the updated `Track`.
        -   Broadcast a `reload_tracks` SSE event.
-   **Part 5: Queue Monitoring API**
    -   Create `src/mus/application/dtos/monitoring.py`. Define a `QueueStatsDTO` with `name: str` and `jobs: int`.
    -   Create `src/mus/infrastructure/api/routers/monitoring_router.py`.
    -   Implement `GET /api/v1/monitoring/queues` which returns a `List[QueueStatsDTO]`. It should get the job counts for `high_priority` and `low_priority` queues from `src/mus/util/queue_utils.py`.
-   **Part 6: Integration and Testing**
    -   In `src/mus/main.py`, add the new `history_router` and `monitoring_router`.
    -   In `src/mus/infrastructure/database.py`, ensure the `TrackHistory` table is created by adding it to `SQLModel.metadata.create_all`.
    -   Write comprehensive tests for the new repositories, worker logic, and API endpoints in the `backend/tests/` directory.
**Filepaths to Create**: `src/mus/domain/entities/track_history.py`, `src/mus/application/dtos/track_history.py`, `src/mus/infrastructure/persistence/sqlite_track_history_repository.py`, `src/mus/infrastructure/api/routers/history_router.py`, `src/mus/application/dtos/monitoring.py`, `src/mus/infrastructure/api/routers/monitoring_router.py`
**Filepaths to Modify**: `src/mus/main.py`, `src/mus/service/worker_tasks.py`, `src/mus/util/db_utils.py`, `src/mus/infrastructure/database.py`
**Relevant Make Commands**: `make back-test, make back-lint`