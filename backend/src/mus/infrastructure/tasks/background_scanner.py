import asyncio
import logging
from typing import Optional

from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase

logger = logging.getLogger(__name__)


class PeriodicScanner:
    def __init__(self, scan_use_case: ScanTracksUseCase, scan_interval_seconds: int):
        self.scan_use_case = scan_use_case
        self._scan_interval = scan_interval_seconds
        self._task: Optional[asyncio.Task] = None
        self._stop_event = asyncio.Event()

    async def start(self) -> None:
        if self._task is None or self._task.done():
            self._stop_event.clear()
            self._task = asyncio.create_task(self._run_periodic_scan())

    async def stop(self) -> None:
        if self._task and not self._task.done():
            self._stop_event.set()
            try:
                await asyncio.wait_for(self._task, timeout=10)
            except asyncio.TimeoutError:
                self._task.cancel()
            finally:
                self._task = None

    async def _run_periodic_scan(self) -> None:
        while not self._stop_event.is_set():
            try:
                await self.scan_use_case.scan_directory()
            except Exception as e:
                logger.error(f"Scan error: {e}")

            try:
                await asyncio.wait_for(
                    self._stop_event.wait(), timeout=self._scan_interval
                )
                break
            except asyncio.TimeoutError:
                continue
