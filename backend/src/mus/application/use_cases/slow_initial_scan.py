import asyncio
import logging
import time
from os import cpu_count

from src.mus.application.use_cases.process_track_metadata import (
    process_slow_metadata_for_track,
)
from src.mus.domain.entities.track import ProcessingStatus
from src.mus.util.db_utils import get_track_by_id, get_tracks_by_status, update_track

logger = logging.getLogger(__name__)

CONCURRENCY_LIMIT = (cpu_count() or 1) * 2


async def _process_track_with_semaphore(track_id: int, semaphore: asyncio.Semaphore):
    async with semaphore:
        try:
            processed_track = await process_slow_metadata_for_track(track_id)
            if processed_track:
                processed_track.processing_status = ProcessingStatus.COMPLETE
                processed_track.last_error = None
                await update_track(processed_track)
            return track_id, None
        except Exception as e:
            logger.error(f"Failed processing track_id {track_id}: {e}", exc_info=False)
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
    async def execute(self):
        logger.info("Phase 2: Slow metadata processing starting...")

        pending_tracks = await get_tracks_by_status(ProcessingStatus.PENDING)
        if not pending_tracks:
            logger.info("No pending tracks to process.")
            return

        logger.info(
            f"Processing {len(pending_tracks)} pending tracks with concurrency limit of {CONCURRENCY_LIMIT}..."
        )

        semaphore = asyncio.Semaphore(CONCURRENCY_LIMIT)
        tasks = [
            _process_track_with_semaphore(track.id, semaphore)
            for track in pending_tracks
            if track.id is not None
        ]

        results = await asyncio.gather(*tasks)

        success_count = sum(1 for _, error in results if error is None)
        error_count = len(results) - success_count

        logger.info(
            f"Slow scan complete. Succeeded: {success_count}, Failed: {error_count}"
        )
