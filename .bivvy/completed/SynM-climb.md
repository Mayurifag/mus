<Climb>
  <header>
    <id>SynM</id>
    <type>task</type>
    <description>Fix SQLAlchemy transaction error during track processing, modify music scanning to be synchronous for debugging, and update track `added_at` to use file modification time.</description>
  </header>
  <newDependencies>None</newDependencies>
  <prerequisitChanges>
    - Existing asynchronous scanning setup during application startup (`lifespan` manager in `backend/src/mus/main.py`).
    - Existing `Track` entity (`backend/src/mus/domain/entities/track.py`) with an `added_at` field.
    - Existing `ScanTracksUseCase` (`backend/src/mus/application/use_cases/scan_tracks_use_case.py`) responsible for metadata extraction and database operations via `SQLiteTrackRepository`.
    - Existing `SQLiteTrackRepository` (`backend/src/mus/infrastructure/persistence/sqlite_track_repository.py`) with `upsert_track` and `set_cover_flag` methods that currently manage their own commits.
  </prerequisitChanges>
  <relevantFiles>
    - `backend/src/mus/main.py`
    - `backend/src/mus/domain/entities/track.py`
    - `backend/src/mus/application/use_cases/scan_tracks_use_case.py`
    - `backend/src/mus/infrastructure/persistence/sqlite_track_repository.py`
    - `backend/tests/test_main_startup.py`
    - `backend/tests/application/test_scan_tracks_use_case.py`
    - `backend/tests/persistence/test_sqlite_track_repository.py`
  </relevantFiles>
  <everythingElse>
    ## Task Overview

    *   **Purpose Statement**: To resolve a critical `sqlalchemy.exc.InvalidRequestError` during track processing, enhance the development and debugging experience by making the initial music scanning process synchronous, and improve data accuracy by ensuring the `added_at` timestamp for tracks reflects their last modification time on disk.
    *   **Problem Being Solved**:
        *   **Critical Error:** `sqlalchemy.exc.InvalidRequestError: Can't operate on closed transaction inside context manager.` This error occurs because `SQLiteTrackRepository` methods (`upsert_track`, `set_cover_flag`) perform explicit commits (`await self.session.commit()`) while being called from within a `async with session.begin():` block in `ScanTracksUseCase`. This creates conflicting transaction scopes.
        *   Asynchronous scanning during startup (e.g., via `make om-up`) can obscure logs and error messages, making debugging difficult.
        *   The current `added_at` field for tracks likely reflects the time of scan/insertion, not the actual last modification time of the music file itself, which is more relevant metadata.
    *   **Success Metrics**:
        *   The `sqlalchemy.exc.InvalidRequestError` during track processing is eliminated. Track scanning completes without transaction-related errors.
        *   When starting the backend server (e.g., `make om-up`), the music scanning process blocks server startup, and its logs/errors are visible in the console.
        *   The `Track.added_at` field in the database correctly stores the Unix timestamp of the music file's last modification time.
        *   This `added_at` value is correctly updated if a file is modified and rescanned.
        *   All relevant unit and integration tests pass.
        *   `make ci` passes.

    ## Requirements

    #### Functional Requirements

    1.  **Transaction Error Resolution**:
        *   The `SQLiteTrackRepository` methods (`upsert_track`, `set_cover_flag`) shall not perform their own `session.commit()`.
        *   Transaction management for these operations shall be handled by the calling context, specifically the `async with self.track_repository.session.begin():` block within `ScanTracksUseCase._process_batch`.
    2.  **Synchronous Startup Scanning**:
        *   The music scanning process initiated during the FastAPI application's `lifespan` event in `backend/src/mus/main.py` shall be executed synchronously.
        *   The server must wait for the scanning to complete before becoming fully available.
    3.  **File Modification Time for `added_at`**:
        *   The `ScanTracksUseCase` (specifically, the `_extract_metadata_sync` method or equivalent) in `backend/src/mus/application/use_cases/scan_tracks_use_case.py` must extract the last modification time (mtime) of each music file.
        *   The `Track` entity defined in `backend/src/mus/domain/entities/track.py` shall have its `added_at` field store this mtime as an integer (Unix timestamp). The `default_factory` currently setting `added_at` to scan time must be removed.
        *   The `SQLiteTrackRepository.upsert_track` method in `backend/src/mus/infrastructure/persistence/sqlite_track_repository.py` must ensure that:
            *   For new tracks, `added_at` is set to the file's mtime.
            *   For existing tracks (conflict on `file_path`), if the file's mtime has changed since the last scan, the `added_at` field is updated to the new mtime.

    #### Technical Requirements

    *   SQLAlchemy session and transaction management best practices must be followed.
    *   Python's `os.path.getmtime()` function should be used to retrieve the file modification time.
    *   The `await` keyword must be used in `backend/src/mus/main.py` to make the `run_scan()` call synchronous within the `lifespan` manager.
    *   SQLModel field definitions and SQLAlchemy dialect-specific UPSERT logic must be updated to reflect the change in `added_at` handling.

    ## Development Details

    *   **`backend/src/mus/infrastructure/persistence/sqlite_track_repository.py`** (Transaction Fix):
        *   In `upsert_track(self, track_data: Track) -> Track`: Remove `await self.session.commit()`.
        *   In `set_cover_flag(self, track_id: Optional[int], has_cover: bool) -> None`: Remove `await self.session.commit()`.
    *   **`backend/src/mus/main.py`** (Synchronous Scan):
        *   Modify the `lifespan` function. Change `asyncio.create_task(run_scan()())` to `await run_scan()()`.
    *   **`backend/src/mus/domain/entities/track.py`** (`added_at` Change):
        *   Change `added_at: int = Field(default_factory=lambda: int(time.time()))` to `added_at: int`.
        *   Remove `import time` if it's no longer used.
    *   **`backend/src/mus/application/use_cases/scan_tracks_use_case.py`** (`added_at` Change):
        *   Ensure `import os` is present.
        *   In `_extract_metadata_sync(self, file_path: Path) -> Dict[str, Any]`:
            *   Add `metadata["mtime"] = int(os.path.getmtime(file_path))` after successfully loading the audio file information.
        *   When instantiating `Track` objects (e.g., in `_process_batch`), use `added_at=metadata["mtime"]`.
    *   **`backend/src/mus/infrastructure/persistence/sqlite_track_repository.py`** (`added_at` Change):
        *   In `upsert_track(self, track_data: Track) -> Track`:
            *   The `track_dict` prepared for `sqlite_upsert(Track).values(**track_dict)` should naturally include `added_at` from `track_data` since it's a direct field now.
            *   The `stmt.on_conflict_do_update(...)` clause's `set_` dictionary must include `added_at=stmt.excluded.added_at`.

    ## Testing Approach

    *   **Transaction Error Resolution**:
        *   Run `make om-up` and observe the logs. Verify that the `sqlalchemy.exc.InvalidRequestError` no longer occurs during track processing and that tracks are successfully added/updated in the database.
        *   Update `backend/tests/persistence/test_sqlite_track_repository.py`: Ensure tests for `upsert_track` and `set_cover_flag` explicitly manage transactions if they previously relied on the methods committing themselves. For example, wrap repository calls in `async with session.begin(): ... await session.commit()` or just `await session.commit()` after the call if the test session is already managed transactionally.
    *   **Synchronous Scanning**:
        *   Manually run `make om-up` and observe that the server log output includes scanning progress/errors and that the server only becomes ready after the scan messages appear.
        *   Update `backend/tests/test_main_startup.py`:
            *   Remove mocks and assertions for `asyncio.create_task` related to `run_scan`.
            *   Ensure tests still verify that `ScanTracksUseCase.scan_directory` (or the `run_scan` wrapper) is called and awaited during startup.
    *   **`added_at` based on File Modification Time**:
        *   **`ScanTracksUseCase._extract_metadata_sync`**:
            *   Add/update unit tests in `backend/tests/application/test_scan_tracks_use_case.py`.
            *   Mock `os.path.getmtime` to return a fixed timestamp.
            *   Verify that the `metadata` dictionary returned by `_extract_metadata_sync` contains the correct `mtime` value.
        *   **`SQLiteTrackRepository.upsert_track`**:
            *   Add/update unit/integration tests in `backend/tests/persistence/test_sqlite_track_repository.py`.
            *   Test Case 1 (New Insert): Create a `Track` object with a specific `added_at` (simulating mtime). Call `upsert_track`. Retrieve the track and verify its `added_at` matches the provided mtime.
            *   Test Case 2 (Update with different mtime): Insert a track with `file_path="p1"` and `added_at=T1`. Then, call `upsert_track` again for `file_path="p1"` but with `added_at=T2` (where `T2 != T1`). Retrieve the track and verify `added_at` is `T2`.
            *   Test Case 3 (Update with same mtime): Insert a track with `file_path="p1"` and `added_at=T1`. Then, call `upsert_track` again for `file_path="p1"` with `added_at=T1`. Retrieve the track and verify `added_at` is still `T1`.
    *   **General**:
        *   Run `make ci` to ensure all existing and new tests pass, and linters/formatters are satisfied.
  </everythingElse>
</Climb>
