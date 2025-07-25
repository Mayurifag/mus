import asyncio
import logging
from concurrent.futures import ProcessPoolExecutor

from src.mus.application.use_cases.process_track_metadata import (
    process_slow_metadata_for_track,
)
from src.mus.core.redis import reset_redis_pool_after_fork
from src.mus.domain.entities.track import ProcessingStatus
from src.mus.util.db_utils import get_tracks_by_status

logger = logging.getLogger(__name__)


def _process_pool_initializer():
    """Initialize worker processes by resetting the Redis connection pool."""
    reset_redis_pool_after_fork()


def _run_in_new_loop(track_id: int):
    return asyncio.run(process_slow_metadata_for_track(track_id))


class SlowInitialScanUseCase:
    async def execute(self):
        logger.info("Phase 2: Slow metadata processing starting...")

        pending_tracks = await get_tracks_by_status(ProcessingStatus.PENDING)
        if not pending_tracks:
            logger.info("No pending tracks to process.")
            return

        logger.info(f"Processing {len(pending_tracks)} pending tracks...")

        loop = asyncio.get_running_loop()
        with ProcessPoolExecutor(initializer=_process_pool_initializer) as executor:
            tasks = [
                loop.run_in_executor(executor, _run_in_new_loop, track.id)
                for track in pending_tracks
                if track.id is not None
            ]

            results = await asyncio.gather(*tasks, return_exceptions=True)

            success_count = sum(1 for r in results if not isinstance(r, Exception))
            error_count = len(results) - success_count

            logger.info(
                f"Slow scan complete. Succeeded: {success_count}, Failed: {error_count}"
            )
            if error_count > 0:
                for i, res in enumerate(results):
                    if isinstance(res, Exception):
                        track_id = pending_tracks[i].id
                        logger.warning(f"Failed processing track_id {track_id}: {res}")
