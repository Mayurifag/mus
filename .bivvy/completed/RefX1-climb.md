<Climb>
  <header>
    <id>RefX1</id>
    <type>refactor</type>
    <description>Refactor SSE broadcasting to be non-blocking, optimize ScanTracksUseCase with batched database commits and simplified SSE strategy, and adjust mtime fallback logic for improved performance and reliability.</description>
  </header>
  <newDependencies>None</newDependencies>
  <prerequisitChanges>
    - Existing `ScanTracksUseCase` in `backend/src/mus/application/use_cases/scan_tracks_use_case.py`.
    - Existing `broadcast_sse_event` function in `backend/src/mus/infrastructure/api/sse_handler.py`.
    - Existing test suite for the application layer.
  </prerequisitChanges>
  <relevantFiles>
    - `backend/src/mus/application/use_cases/scan_tracks_use_case.py`
    - `backend/src/mus/infrastructure/api/sse_handler.py`
    - `backend/tests/application/test_scan_tracks_use_case.py`
    - `backend/tests/api/test_sse_handler.py`
  </relevantFiles>
  <everythingElse>
    ## Problem Statement

    The current implementation exhibits several issues:
    1.  **Blocking SSE Broadcast Invocation:** Calls to `broadcast_sse_event` from `ScanTracksUseCase` are awaited, potentially slowing down the scan process if SSE dispatch is slow.
    2.  **Potentially Blocking Internal SSE Dispatch:** The `broadcast_sse_event` function itself awaits `queue.put()` for each client, meaning a slow or full queue for one client could delay message delivery to subsequent clients within the same event broadcast.
    3.  **Long Server Response Times (up to 15s):** Observed API response delays are likely due to `ScanTracksUseCase` holding a single, long-running database transaction for the entire scan operation. This can block other API requests that need database access.
    4.  **Complex/Noisy SSE Strategy:** Multiple SSE events (per-track, per-batch progress) are sent during a scan, which can be verbose and might not be efficiently handled or desired by the client.
    5.  **`mtime` Fallback:** The current fallback for `mtime` to `datetime.now()` in `_extract_metadata_sync` when `os.path.getmtime()` fails can lead to incorrect timestamping and re-processing of files.

    ## Solution Scope

    This refactoring effort will address the identified problems through the following changes:

    1.  **Non-Blocking SSE Invocation:** All calls to `broadcast_sse_event` from `ScanTracksUseCase` will be changed to use `asyncio.create_task`, making them fire-and-forget and preventing the scanner from waiting for SSE dispatch.
    2.  **Non-Blocking Internal SSE Dispatch:** The `broadcast_sse_event` function will be modified to use `asyncio.create_task` for each `queue.put()` call, ensuring that sending to one client's queue does not block sending to others.
    3.  **Batched Database Commits in `ScanTracksUseCase`:** The `scan_directory` method in `ScanTracksUseCase` will be refactored to process files in smaller batches. Each batch will have its database operations (track upserts, cover flag updates) committed in its own short-lived transaction.
    4.  **Simplified SSE Strategy:** Intermediate SSE events (per-track and per-batch progress) from `ScanTracksUseCase` will be removed. Only a single, final summary SSE event will be dispatched after the entire scan operation concludes. This event will carry a generic completion message and will include `action_key: 'reload_tracks'` if any tracks were processed (added/updated), signaling the client to refresh its data.
    5.  **`mtime` Fallback Adjustment:** If `os.path.getmtime()` fails during metadata extraction in `_extract_metadata_sync`, the system will log an error and use `0` as the `mtime` value for that track.

    ## Success Metrics

    *   All invocations of `broadcast_sse_event` from `ScanTracksUseCase` are confirmed to be non-blocking (using `asyncio.create_task`).
    *   The internal loop of `broadcast_sse_event` uses `asyncio.create_task` for `queue.put()`, ensuring non-blocking dispatch to individual clients.
    *   `ScanTracksUseCase` performs database commits in batches, verifiable through code review and potentially logging/testing.
    *   `ScanTracksUseCase` sends only one final summary SSE event upon completion, with an appropriate message and `action_key: 'reload_tracks'` if changes occurred. Intermediate SSEs related to scan progress are removed.
    *   When `os.path.getmtime()` fails, the `mtime` for the affected track is set to `0`, and an error is logged.
    *   Observed server response times for concurrent API requests (e.g., fetching tracks, player state) are improved or no longer hang for extended periods during background scans.
    *   All relevant unit and integration tests pass.
    *   The `make ci` command completes successfully without errors.
  </everythingElse>
</Climb>