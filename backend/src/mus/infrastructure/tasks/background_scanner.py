import asyncio
import logging
from typing import Callable, Coroutine, Any, Dict, Optional

from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.application.dtos.scan import ScanResponseDTO
from src.mus.infrastructure.api.sse_handler import broadcast_sse_event

logger = logging.getLogger(__name__)


class PeriodicScanner:
    def __init__(
        self,
        scan_use_case: ScanTracksUseCase,
        scan_interval_seconds: int,
        broadcast_func: Callable[
            [Dict[str, Any]], Coroutine[Any, Any, None]
        ] = broadcast_sse_event,
    ):
        self.scan_use_case = scan_use_case
        self._broadcast_sse_event = broadcast_func
        self._scan_interval = scan_interval_seconds
        self._task: Optional[asyncio.Task] = None
        self._stop_event = asyncio.Event()

    async def start(self) -> None:
        if self._task is None or self._task.done():
            self._stop_event.clear()
            self._task = asyncio.create_task(self._run_periodic_scan())
            logger.info("Periodic scanner started.")

    async def stop(self) -> None:
        if self._task and not self._task.done():
            self._stop_event.set()
            try:
                await asyncio.wait_for(self._task, timeout=self._scan_interval + 10)
            except asyncio.TimeoutError:
                logger.warning(
                    "Periodic scanner task did not stop gracefully, cancelling."
                )
                self._task.cancel()
            except asyncio.CancelledError:
                logger.info("Periodic scanner task was cancelled during stop.")
            finally:
                self._task = None
            logger.info("Periodic scanner stopped.")
        else:
            logger.info("Periodic scanner not running or already stopped.")

    async def _perform_single_scan(self) -> Optional[ScanResponseDTO]:
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
            return scan_result
        except Exception as e:
            logger.error(f"Error during scan directory operation: {e}", exc_info=True)
            return None

    async def _wait_for_next_scan(self) -> bool:
        if self._stop_event.is_set():
            return False

        try:
            wait_task = asyncio.create_task(self._stop_event.wait())
            try:
                await asyncio.wait_for(wait_task, timeout=self._scan_interval)
                return False
            except asyncio.TimeoutError:
                return True
            except asyncio.CancelledError:
                logger.info("Periodic scan wait cancelled, stopping.")
                return False
            finally:
                if not wait_task.done():
                    wait_task.cancel()
                    try:
                        await wait_task
                    except asyncio.CancelledError:
                        pass
        except Exception as e:
            logger.error(f"Error during wait: {e}", exc_info=True)
            return False

    async def _run_periodic_scan(self) -> None:
        try:
            while not self._stop_event.is_set():
                await self._perform_single_scan()
                should_continue = await self._wait_for_next_scan()
                if not should_continue:
                    break
            logger.info("Periodic scan loop ended.")
        except Exception as e:
            logger.error(f"Unexpected error in periodic scan loop: {e}", exc_info=True)
        finally:
            logger.info("Periodic scan loop exited.")
