import asyncio
import logging
import traceback
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
    """
    Wrapper to run the async metadata processing in a new event loop
    in a separate process. Captures and returns exceptions.
    """
    try:
        # Running the async function and getting the result
        return asyncio.run(process_slow_metadata_for_track(track_id))
    except Exception as e:
        # Capture the exception and its traceback to be returned to the parent
        tb_str = traceback.format_exc()
        return (e, tb_str)


class SlowInitialScanUseCase:
    async def execute(self):
        logger.info("Phase 2: Slow metadata processing starting...")

        pending_tracks = await get_tracks_by_status(ProcessingStatus.PENDING)
        if not pending_tracks:
            logger.info("No pending tracks to process.")
            return

        logger.info(
            f"Processing {len(pending_tracks)} pending tracks using up to 2 worker processes."
        )

        loop = asyncio.get_running_loop()
        with ProcessPoolExecutor(
            max_workers=2, initializer=_process_pool_initializer
        ) as executor:
            tasks = [
                loop.run_in_executor(executor, _run_in_new_loop, track.id)
                for track in pending_tracks
                if track.id is not None
            ]

            results = await asyncio.gather(*tasks)

            success_count = 0
            error_count = 0

            for i, res in enumerate(results):
                if isinstance(res, tuple) and len(res) == 2 and isinstance(res[0], Exception):
                    error_count += 1
                    track_id = pending_tracks[i].id
                    exc, tb_str = res
                    logger.error(
                        f"Exception in child process for track_id {track_id}: {exc}\n{tb_str}"
                    )
                else:
                    success_count += 1

            logger.info(
                f"Slow scan complete. Succeeded: {success_count}, Failed: {error_count}"
            )
