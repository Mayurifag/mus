<Climb>
  <header>
    <id>F4gH</id>
    <type>task</type>
    <description>Implement a conditional, one-time startup delay for `PeriodicScanner` to improve initial server responsiveness (without altering test behavior), and refactor `CoverProcessor` to run `pyvips` operations in a thread pool for better responsiveness during scans. Consolidate related tests and CI check into two moves.</description>
  </header>
  <newDependencies>None.</newDependencies>
  <prerequisitChanges>None.</prerequisitChanges>
  <relevantFiles>
    - `backend/src/mus/infrastructure/tasks/background_scanner.py`
    - `backend/src/mus/config.py`
    - `backend/src/mus/infrastructure/scanner/cover_processor.py`
    - `backend/tests/infrastructure/test_cover_processor.py`
    - `backend/tests/infrastructure/test_enhanced_cover_processor.py`
  </relevantFiles>
  <everythingElse>
    **Objective 1: Faster Server Startup (Conditional Delay - No Test Impact)**
    The `PeriodicScanner` will have its first scan operation delayed slightly upon server startup, but only if the application is not running in a 'test' environment. This delay aims to improve initial server responsiveness. This change is designed such that existing tests for `PeriodicScanner` should not require modification, as the delay is conditional and tests operate in `APP_ENV="test"`.

    **Detailed Changes for `PeriodicScanner` (Move 1):**
    1.  **Modify `_run_periodic_scan`:** In `backend/src/mus/infrastructure/tasks/background_scanner.py`, within the `PeriodicScanner` class's `_run_periodic_scan` method:
        *   Import `asyncio` and `from src.mus.config import settings`.
        *   Immediately before the `while not self._stop_event.is_set():` loop, add the conditional sleep: `if settings.APP_ENV != "test": await asyncio.sleep(0.5)`.
        *   No `_first_scan_done` flag is required as this code block will execute only once when `_run_periodic_scan` is first invoked.
    2.  **No Test Changes for this move:** The existing tests in `backend/tests/infrastructure/tasks/test_background_scanner.py` are expected to pass without modification because the sleep condition `settings.APP_ENV != "test"` will be false during test execution (where `APP_ENV` is typically "test").

    **Objective 2: Improve Scan Responsiveness (pyvips via Thread Pool) & CI (Move 2)**
    CPU-intensive `pyvips` operations in `CoverProcessor.process_and_save_cover` will be offloaded to a separate thread using `asyncio.to_thread` to prevent blocking the main event loop during cover processing.

    **Detailed Changes for `CoverProcessor` (Move 2):**
    1.  **Create Synchronous Helper:** In `backend/src/mus/infrastructure/scanner/cover_processor.py` (`CoverProcessor` class):
        *   Create a new private synchronous method, e.g., `_save_cover_sync(self, track_id: int, cover_data_cleaned: bytes, original_path: Path, small_path: Path, file_path: Optional[Path] = None) -> bool`.
        *   Move all `pyvips` related logic (`VipsImage.new_from_buffer`, `image.webpsave`, `image.thumbnail_image`, `thumbnail.webpsave`, and their try-except block) into this new method. It should return `True` on success, `False` on failure, and perform logging as before. The `file_path` is for logging.
    2.  **Modify `process_and_save_cover`:**
        *   The `async def process_and_save_cover` method will now invoke the synchronous helper: `return await asyncio.to_thread(self._save_cover_sync, track_id, cleaned_data, original_path, small_path, file_path)`.
        *   Ensure `asyncio` is imported.
    3.  **Update Tests:** In `backend/tests/infrastructure/test_cover_processor.py` (and/or `test_enhanced_cover_processor.py` if more relevant):
        *   Ensure tests for `process_and_save_cover` are `async` (`pytest.mark.asyncio`).
        *   Mock `asyncio.to_thread` to assert that `_save_cover_sync` is called with correct arguments.
        *   Test how `process_and_save_cover` handles scenarios where the mocked `_save_cover_sync` returns `True`, `False`, or raises an exception.
    4.  **Run CI Checks:** Execute `make ci` from the project root directory. All checks (linting, formatting, tests) must pass without any errors or warnings.

    Python code changes must adhere to `.cursor/rules/python.mdc`. Makefile commands must follow `.cursor/rules/makefile.mdc`.
  </everythingElse>
</Climb>