import asyncio
import logging
import traceback
import os
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
    logger.info(f"[CHILD-INIT] Worker process {os.getpid()} initializing...")
    try:
        reset_redis_pool_after_fork()
        logger.info(
            f"[CHILD-INIT] Worker process {os.getpid()} Redis pool reset complete"
        )
    except Exception as e:
        logger.error(
            f"[CHILD-INIT] Worker process {os.getpid()} failed to initialize: {e}",
            exc_info=True,
        )
        raise


def _run_in_new_loop(track_id: int):
    """
    Wrapper to run the async metadata processing in a new event loop
    in a separate process. Captures and returns exceptions.
    """
    pid = os.getpid()
    logger.info(f"[CHILD-{pid}] Starting _run_in_new_loop for track {track_id}")

    try:
        # Log memory usage if available
        try:
            import psutil

            process = psutil.Process(pid)
            memory_info = process.memory_info()
            logger.info(
                f"[CHILD-{pid}] Memory usage before processing: RSS={memory_info.rss / 1024 / 1024:.1f}MB, VMS={memory_info.vms / 1024 / 1024:.1f}MB"
            )
        except ImportError:
            logger.info(f"[CHILD-{pid}] psutil not available, cannot log memory usage")
        except Exception as e:
            logger.warning(f"[CHILD-{pid}] Failed to get memory info: {e}")

        logger.info(f"[CHILD-{pid}] About to call asyncio.run for track {track_id}")
        result = asyncio.run(process_slow_metadata_for_track(track_id))
        logger.info(
            f"[CHILD-{pid}] Successfully completed processing for track {track_id}"
        )
        return result

    except KeyboardInterrupt:
        logger.error(
            f"[CHILD-{pid}] Process interrupted (SIGINT) while processing track {track_id}"
        )
        return (KeyboardInterrupt("Process interrupted"), traceback.format_exc())
    except MemoryError as e:
        logger.error(
            f"[CHILD-{pid}] Memory error while processing track {track_id}: {e}"
        )
        return (e, traceback.format_exc())
    except Exception as e:
        # Capture the exception and its traceback to be returned to the parent
        logger.error(
            f"[CHILD-{pid}] Exception in _run_in_new_loop for track {track_id}: {e}",
            exc_info=True,
        )
        tb_str = traceback.format_exc()
        return (e, tb_str)
    finally:
        logger.info(f"[CHILD-{pid}] Exiting _run_in_new_loop for track {track_id}")


class SlowInitialScanUseCase:
    async def execute(self):
        logger.info("[PARENT] Phase 2: Slow metadata processing starting...")

        pending_tracks = await get_tracks_by_status(ProcessingStatus.PENDING)
        if not pending_tracks:
            logger.info("[PARENT] No pending tracks to process.")
            return

        logger.info(
            f"[PARENT] Processing {len(pending_tracks)} pending tracks using up to 2 worker processes."
        )

        # Log track IDs being processed
        track_ids = [track.id for track in pending_tracks if track.id is not None]
        logger.info(f"[PARENT] Track IDs to process: {track_ids}")

        loop = asyncio.get_running_loop()
        logger.info("[PARENT] Creating ProcessPoolExecutor with 2 workers...")

        try:
            with ProcessPoolExecutor(
                max_workers=2, initializer=_process_pool_initializer
            ) as executor:
                logger.info("[PARENT] ProcessPoolExecutor created successfully")

                # Create tasks with explicit logging
                tasks = []
                for track in pending_tracks:
                    if track.id is not None:
                        logger.info(f"[PARENT] Submitting track {track.id} to executor")
                        task = loop.run_in_executor(
                            executor, _run_in_new_loop, track.id
                        )
                        tasks.append(task)

                logger.info(
                    f"[PARENT] Submitted {len(tasks)} tasks to executor, waiting for completion..."
                )

                # Use asyncio.gather with return_exceptions=True to catch all issues
                results = await asyncio.gather(*tasks, return_exceptions=True)
                logger.info(
                    f"[PARENT] All tasks completed, processing {len(results)} results..."
                )

                success_count = 0
                error_count = 0

                for i, res in enumerate(results):
                    track_id = pending_tracks[i].id
                    logger.info(
                        f"[PARENT] Processing result {i + 1}/{len(results)} for track {track_id}"
                    )

                    # Check for asyncio.gather exceptions first
                    if isinstance(res, Exception):
                        error_count += 1
                        logger.error(
                            f"[PARENT] Asyncio exception for track_id {track_id}: {type(res).__name__}: {res}",
                            exc_info=res,
                        )
                        continue

                    # Check for child process exceptions (returned as tuples)
                    if (
                        isinstance(res, tuple)
                        and len(res) == 2
                        and isinstance(res[0], Exception)
                    ):
                        error_count += 1
                        exc, tb_str = res
                        logger.error(
                            f"[PARENT] Child process exception for track_id {track_id}: {type(exc).__name__}: {exc}"
                        )
                        logger.error(
                            f"[PARENT] Child process traceback for track_id {track_id}:\n{tb_str}"
                        )

                        # Special handling for memory errors
                        if isinstance(exc, MemoryError):
                            logger.critical(
                                f"[PARENT] MEMORY ERROR detected for track_id {track_id} - this may indicate the 1GB memory limit is being exceeded"
                            )
                        elif isinstance(exc, KeyboardInterrupt):
                            logger.critical(
                                f"[PARENT] PROCESS INTERRUPTED for track_id {track_id} - child process may have been killed by OS"
                            )
                    else:
                        success_count += 1
                        logger.info(f"[PARENT] Track {track_id} processed successfully")

                logger.info(
                    f"[PARENT] Slow scan complete. Succeeded: {success_count}, Failed: {error_count}"
                )

        except Exception as e:
            logger.critical(
                f"[PARENT] Critical error in ProcessPoolExecutor: {e}", exc_info=True
            )
            raise
