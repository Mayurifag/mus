import asyncio
import logging
import time
from os import cpu_count

from src.mus.application.services.permissions_service import PermissionsService
from src.mus.application.use_cases.process_track_metadata import (
    process_slow_metadata_for_track,
)
from src.mus.domain.entities.track import ProcessingStatus
from src.mus.util.db_utils import (
    get_track_by_id,
    get_track_ids_by_status,
    update_track,
)
from src.mus.util.memory import release_process_memory

logger = logging.getLogger(__name__)

CONCURRENCY_LIMIT = (cpu_count() or 1) * 2
BATCH_SIZE = max(CONCURRENCY_LIMIT * 4, 25)


async def _process_track_with_semaphore(
    track_id: int, semaphore: asyncio.Semaphore, permissions_service: PermissionsService
):
    async with semaphore:
        try:
            processed_track = await process_slow_metadata_for_track(
                track_id, permissions_service
            )
            if processed_track:
                processed_track.processing_status = ProcessingStatus.COMPLETE
                processed_track.last_error = None
                await update_track(processed_track)
            else:
                track_to_update = await get_track_by_id(track_id)
                if track_to_update:
                    track_to_update.processing_status = ProcessingStatus.ERROR
                    track_to_update.last_error = {
                        "timestamp": int(time.time()),
                        "job_name": "slow_initial_scan",
                        "error_message": "Track file no longer exists",
                    }
                    await update_track(track_to_update)
                return track_id, RuntimeError("Track file no longer exists")
            return track_id, None
        except Exception as e:
            logger.error(f"Failed processing track_id {track_id}: {e}", exc_info=True)
            track_to_update = await get_track_by_id(track_id)
            if track_to_update:
                track_to_update.processing_status = ProcessingStatus.ERROR
                track_to_update.last_error = {
                    "timestamp": int(time.time()),
                    "job_name": "slow_initial_scan",
                    "error_message": str(e),
                }
                await update_track(track_to_update)
            return track_id, e


class SlowInitialScanUseCase:
    def __init__(self, permissions_service: PermissionsService):
        self.permissions_service = permissions_service

    async def execute(self):
        logger.info("Phase 2: Slow metadata processing starting...")

        pending_track_ids = await get_track_ids_by_status(
            ProcessingStatus.PENDING, BATCH_SIZE
        )
        if not pending_track_ids:
            logger.info("No pending tracks to process.")
            return

        logger.info(
            f"Processing pending tracks with concurrency limit of {CONCURRENCY_LIMIT}..."
        )

        success_count = 0
        error_count = 0

        while pending_track_ids:
            semaphore = asyncio.Semaphore(CONCURRENCY_LIMIT)
            tasks = [
                _process_track_with_semaphore(
                    track_id, semaphore, self.permissions_service
                )
                for track_id in pending_track_ids
            ]

            results = await asyncio.gather(*tasks)
            success_count += sum(1 for _, error in results if error is None)
            error_count += sum(1 for _, error in results if error is not None)
            del tasks, results

            await asyncio.to_thread(release_process_memory)

            pending_track_ids = await get_track_ids_by_status(
                ProcessingStatus.PENDING, BATCH_SIZE
            )

        logger.info(
            f"Slow scan complete. Succeeded: {success_count}, Failed: {error_count}"
        )
