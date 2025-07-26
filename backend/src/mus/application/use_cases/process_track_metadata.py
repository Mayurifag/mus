import asyncio
import logging
import time
from pathlib import Path
from typing import Optional

from mutagen._file import File as MutagenFile

from src.mus.config import settings
from src.mus.core.redis import set_app_write_lock
from src.mus.domain.entities.track import Track
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.util.db_utils import get_track_by_id, update_track
from src.mus.util.ffprobe_analyzer import get_accurate_duration

logger = logging.getLogger(__name__)


async def process_slow_metadata_for_track(track_id: int) -> Optional[Track]:
    import os

    pid = os.getpid()
    logger.info(
        f"[BREADCRUMB-{pid}] Starting slow metadata processing for track {track_id}"
    )

    try:
        logger.info(
            f"[BREADCRUMB-{pid}] Step 1: Fetching track {track_id} from database"
        )
        track = await get_track_by_id(track_id)
        if not track:
            logger.warning(
                f"[BREADCRUMB-{pid}] Track {track_id} not found in database. Aborting."
            )
            return None
        logger.info(
            f"[BREADCRUMB-{pid}] Step 1 complete: Track {track_id} found in database"
        )

        logger.info(
            f"[BREADCRUMB-{pid}] Step 2: Checking file existence for track {track_id}"
        )
        file_path = Path(track.file_path)
        if not file_path.exists():
            logger.warning(
                f"[BREADCRUMB-{pid}] File not found at path: {file_path}. Aborting."
            )
            return None
        logger.info(f"[BREADCRUMB-{pid}] Step 2 complete: File exists at {file_path}")

        logger.info(
            f"[BREADCRUMB-{pid}] Step 3: Getting file stats for track {track_id}"
        )
        try:
            original_mtime = file_path.stat().st_mtime
            logger.info(
                f"[BREADCRUMB-{pid}] Step 3 complete: Original mtime: {original_mtime}"
            )
        except Exception as e:
            logger.error(
                f"[BREADCRUMB-{pid}] Step 3 failed: Error getting file stats: {e}"
            )
            raise

        logger.info(
            f"[BREADCRUMB-{pid}] Step 4: Loading audio file with Mutagen for track {track_id}"
        )
        try:
            audio = MutagenFile(file_path, easy=False)
            logger.info(
                f"[BREADCRUMB-{pid}] Step 4 complete: Audio file loaded successfully"
            )
        except Exception as e:
            logger.error(
                f"[BREADCRUMB-{pid}] Step 4 failed: Error loading audio file: {e}"
            )
            raise

        logger.info(
            f"[BREADCRUMB-{pid}] Step 5: Checking ID3 standardization for track {track_id}"
        )
        try:
            if audio and _needs_id3_standardization(audio):
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 5a: Standardizing ID3 tags to v2.3"
                )
                await set_app_write_lock(str(file_path))
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 5b: App write lock set, saving audio file"
                )
                audio.save(v2_version=3)
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 5c: Audio file saved, restoring mtime"
                )
                os.utime(file_path, (original_mtime, original_mtime))
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 5 complete: ID3 tags standardized"
                )
            else:
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 5 complete: ID3 tag standardization not needed"
                )
        except Exception as e:
            logger.error(
                f"[BREADCRUMB-{pid}] Step 5 failed: Error during ID3 standardization: {e}"
            )
            raise

        logger.info(
            f"[BREADCRUMB-{pid}] Step 6: Creating CoverProcessor for track {track_id}"
        )
        try:
            cover_processor = CoverProcessor(settings.COVERS_DIR_PATH)
            logger.info(f"[BREADCRUMB-{pid}] Step 6 complete: CoverProcessor created")
        except Exception as e:
            logger.error(
                f"[BREADCRUMB-{pid}] Step 6 failed: Error creating CoverProcessor: {e}"
            )
            raise

        logger.info(
            f"[BREADCRUMB-{pid}] Step 7: Starting parallel duration analysis and cover extraction for track {track_id}"
        )
        try:
            logger.info(f"[BREADCRUMB-{pid}] Step 7a: Creating duration analysis task")
            duration_task = asyncio.to_thread(get_accurate_duration, file_path)
            logger.info(f"[BREADCRUMB-{pid}] Step 7b: Creating cover extraction task")
            cover_task = cover_processor.extract_cover_from_file(file_path)

            logger.info(
                f"[BREADCRUMB-{pid}] Step 7c: Waiting for both tasks to complete"
            )
            accurate_duration, cover_data = await asyncio.gather(
                duration_task, cover_task
            )
            logger.info(
                f"[BREADCRUMB-{pid}] Step 7 complete: Duration={accurate_duration}, Cover data length={len(cover_data) if cover_data else 0}"
            )
        except Exception as e:
            logger.error(
                f"[BREADCRUMB-{pid}] Step 7 failed: Error during parallel processing: {e}"
            )
            raise

        logger.info(
            f"[BREADCRUMB-{pid}] Step 8: Processing results for track {track_id}"
        )
        track_updated = False

        logger.info(
            f"[BREADCRUMB-{pid}] Step 8a: Checking duration update for track {track_id}"
        )
        if accurate_duration > 0 and accurate_duration != track.duration:
            logger.info(
                f"[BREADCRUMB-{pid}] Step 8a: Updating duration from {track.duration} to {accurate_duration}"
            )
            track.duration = accurate_duration
            track_updated = True
        else:
            logger.info(f"[BREADCRUMB-{pid}] Step 8a: No duration update needed")

        logger.info(
            f"[BREADCRUMB-{pid}] Step 8b: Processing cover data for track {track_id}"
        )
        try:
            if cover_data:
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 8b: Found cover data ({len(cover_data)} bytes), processing and saving"
                )
                new_has_cover = await cover_processor.process_and_save_cover(
                    track_id, cover_data, file_path
                )
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 8b complete: Cover processing finished. Has cover: {new_has_cover}"
                )
            else:
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 8b complete: No cover data found in file"
                )
                new_has_cover = False
        except Exception as e:
            logger.error(
                f"[BREADCRUMB-{pid}] Step 8b failed: Error processing cover data: {e}"
            )
            raise

        logger.info(
            f"[BREADCRUMB-{pid}] Step 8c: Checking cover status update for track {track_id}"
        )
        if track.has_cover != new_has_cover:
            logger.info(
                f"[BREADCRUMB-{pid}] Step 8c: Updating has_cover from {track.has_cover} to {new_has_cover}"
            )
            track.has_cover = new_has_cover
            track_updated = True
        else:
            logger.info(f"[BREADCRUMB-{pid}] Step 8c: No cover status update needed")

        logger.info(
            f"[BREADCRUMB-{pid}] Step 9: Finalizing track updates for track {track_id}"
        )
        try:
            if track_updated:
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 9a: Track has updates, saving to database"
                )
                track.updated_at = int(time.time())
                await update_track(track)
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 9a complete: Track successfully updated in database"
                )
            else:
                logger.info(
                    f"[BREADCRUMB-{pid}] Step 9a: No metadata changes for track"
                )
        except Exception as e:
            logger.error(
                f"[BREADCRUMB-{pid}] Step 9a failed: Error updating track in database: {e}"
            )
            raise

        logger.info(
            f"[BREADCRUMB-{pid}] SUCCESS: Finished slow metadata processing for track {track_id}"
        )
        return track

    except Exception as e:
        logger.error(
            f"[BREADCRUMB-{pid}] FATAL ERROR: Unhandled exception in process_slow_metadata_for_track for track {track_id}: {e}",
            exc_info=True,
        )
        raise


def _needs_id3_standardization(audio: MutagenFile) -> bool:
    if hasattr(audio, "tags") and audio.tags:
        if hasattr(audio.tags, "version") and audio.tags.version == (2, 3, 0):
            return False
        return True
    return False
