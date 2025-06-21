# Foundational Backend Refactor for Robust Processing

## User Problem
The current backend file processing is monolithic and not robust for file moves. The database is not tuned for concurrent access, leading to potential performance bottlenecks. The system lacks a clear way to track the processing status of individual files.

## High-Level Solution
Implement foundational changes to make the backend more robust and prepare it for an event-driven architecture. This involves enhancing the database schema for better file tracking, tuning SQLite for concurrency, and integrating a task queue system (RQ with DragonflyDB) for asynchronous job processing.

## Success Metrics
- The `Track` schema is updated with `inode`, `content_hash`, `processing_status`, and `last_error_message`.
- SQLite connections are configured with WAL mode and a busy timeout.
- RQ and DragonflyDB are integrated for both local development (Docker Compose) and production (Dockerfile, Supervisor).
- The application builds, runs, and passes all CI checks with these new infrastructure components in place.

## Detailed Description
This task establishes the core infrastructure for a more scalable and resilient backend.
1.  **Schema Enhancement**: By adding `inode` and `content_hash`, we can more reliably track files even if they are moved or renamed, and detect duplicates. The `processing_status` and `last_error_message` fields will give us visibility into background job execution for each track.
2.  **SQLite Tuning**: We will enable Write-Ahead Logging (WAL) mode to significantly improve database concurrency, which is crucial for an application with both API requests and background workers accessing the database simultaneously.
3.  **Task Queue Setup**: We will introduce RQ (a simple Python task queue) and DragonflyDB (a high-performance Redis-compatible datastore) to offload long-running tasks like metadata extraction and cover art processing. This task focuses on setting up the services and configuration, not on implementing the tasks themselves.

## Subtasks

### [ ] 1. Enhance Track Schema and Tune SQLite Concurrency
**Description**: Update the `Track` schema for robust file tracking and tune SQLite for better concurrency.
**Details**:
- **Schema Enhancement**:
    - In `backend/src/mus/domain/entities/track.py`, modify the `Track` model.
    - Import `Enum` from `enum`. Define a new `class ProcessingStatus(str, Enum):` with values `PENDING`, `METADATA_DONE`, `ART_DONE`, `COMPLETE`, `ERROR`.
    - Import `Column` and `String` from `sqlalchemy`.
    - Add the following fields to the `Track` class:
        - `inode: Optional[int] = Field(default=None, index=True)`
        - `content_hash: Optional[str] = Field(default=None, index=True)`
        - `processing_status: ProcessingStatus = Field(default=ProcessingStatus.PENDING, sa_column=Column(String(20), nullable=False))`.
        - `last_error_message: Optional[str] = Field(default=None)`.
- **SQLite Tuning**:
    - In `backend/src/mus/infrastructure/database.py`, configure the SQLAlchemy engine to set PRAGMAs for each new connection.
    - Import `event` from `sqlalchemy`.
    - Define a listener for the `connect` event on `engine.sync_engine`: `@event.listens_for(engine.sync_engine, "connect")`.
    - The listener function, `def set_sqlite_pragma(dbapi_connection, connection_record):`, should execute these commands using a cursor:
        - `cursor = dbapi_connection.cursor()`
        - `cursor.execute("PRAGMA journal_mode=WAL")`
        - `cursor.execute("PRAGMA synchronous=NORMAL")`
        - `cursor.execute("PRAGMA busy_timeout=5000")`
        - `cursor.close()`
- **Testing**: After these changes, the database schema will have changed. You must delete any local database files (e.g., `app_data/database/mus.db`) to allow the application to recreate it with the new schema on startup. The test database (`test.db`) should be cleared automatically by the `conftest.py` fixture. Update any tests in `backend/tests/persistence/` that are affected by the new schema fields.
**Filepaths to Modify**: `backend/src/mus/domain/entities/track.py,backend/src/mus/infrastructure/database.py`
**Relevant Make Commands**: `make back-test`