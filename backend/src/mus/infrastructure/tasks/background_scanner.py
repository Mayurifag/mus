import asyncio
import logging

from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.infrastructure.api.sse_handler import broadcast_sse_event

logger = logging.getLogger(__name__)


class PeriodicScanner:
    def __init__(
        self,
        scan_use_case: ScanTracksUseCase,
        scan_interval_seconds: int,
    ):
        self.scan_use_case = scan_use_case
        # Consider making broadcast_sse_event injectable if testing is needed
        self._broadcast_sse_event = broadcast_sse_event
        self._scan_interval = scan_interval_seconds
        self._task: asyncio.Task | None = None
        self._stop_event = asyncio.Event()

    async def start(self):
        if self._task is None or self._task.done():
            self._stop_event.clear()
            self._task = asyncio.create_task(self._run_periodic_scan())
            logger.info("Periodic scanner started.")

    async def stop(self):
        if self._task and not self._task.done():
            self._stop_event.set()
            try:
                # Give the current scan cycle a bit more time than the interval to finish
                await asyncio.wait_for(self._task, timeout=self._scan_interval + 10)
            except asyncio.TimeoutError:
                logger.warning(
                    "Periodic scanner task did not stop gracefully, cancelling."
                )
                self._task.cancel()
            except asyncio.CancelledError:
                logger.info("Periodic scanner task was cancelled during stop.")
            finally:
                self._task = None  # Ensure task is cleared
            logger.info("Periodic scanner stopped.")
        else:
            logger.info("Periodic scanner not running or already stopped.")

    async def _run_periodic_scan(self):
        while not self._stop_event.is_set():
            try:
                logger.info("Periodic scan cycle started.")
                scan_result = await self.scan_use_case.scan_directory()
                logger.info(
                    f"Scan completed: {scan_result.tracks_added} added, "
                    f"{scan_result.tracks_updated} updated, {scan_result.errors} errors."
                )
                await self._broadcast_sse_event(
                    {
                        "type": "scan_completed",
                        "summary": scan_result.model_dump(),
                    }
                )
            except Exception as e:
                # Log and continue, so one failed scan doesn't stop the periodic task
                logger.error(
                    f"Error during scan directory operation: {e}", exc_info=True
                )

            # Wait for the next scan interval or until stop event is set
            if not self._stop_event.is_set():
                try:
                    # Wait for the scan_interval, but break if stop_event is set sooner
                    await asyncio.wait_for(
                        self._stop_event.wait(), timeout=self._scan_interval
                    )
                except asyncio.TimeoutError:
                    pass  # Timeout means it's time for the next scan
                except asyncio.CancelledError:
                    logger.info("Periodic scan wait cancelled, stopping.")
                    break  # Exit loop if wait is cancelled (e.g. during shutdown)
        logger.info("Periodic scan loop ended.")
