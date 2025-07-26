import asyncio
import logging
import os
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
    logger.info(f"[{track_id}] Starting slow metadata processing.")
    track = await get_track_by_id(track_id)
    if not track:
        logger.warning(f"[{track_id}] Track not found in database. Aborting.")
        return None

    file_path = Path(track.file_path)
    if not file_path.exists():
        logger.warning(f"[{track_id}] File not found at path: {file_path}. Aborting.")
        return None

    logger.info(f"[{track_id}] Processing file: {file_path}")

    try:
        original_mtime = file_path.stat().st_mtime
        logger.debug(f"[{track_id}] Original mtime: {original_mtime}")

        audio = MutagenFile(file_path, easy=False)
        if audio and _needs_id3_standardization(audio):
            logger.info(f"[{track_id}] Standardizing ID3 tags to v2.3.")
            await set_app_write_lock(str(file_path))
            audio.save(v2_version=3)
            os.utime(file_path, (original_mtime, original_mtime))
            logger.info(f"[{track_id}] ID3 tags standardized.")
        else:
            logger.info(f"[{track_id}] ID3 tag standardization not needed.")

        cover_processor = CoverProcessor(settings.COVERS_DIR_PATH)

        logger.info(
            f"[{track_id}] Starting duration analysis and cover extraction in parallel."
        )
        duration_task = asyncio.to_thread(get_accurate_duration, file_path)
        cover_task = cover_processor.extract_cover_from_file(file_path)

        accurate_duration, cover_data = await asyncio.gather(duration_task, cover_task)
        logger.info(
            f"[{track_id}] Duration analysis and cover extraction finished."
        )
        logger.debug(
            f"[{track_id}] Accurate duration: {accurate_duration}, Cover data length: {len(cover_data) if cover_data else 0}"
        )

        track_updated = False

        if accurate_duration > 0 and accurate_duration != track.duration:
            logger.info(
                f"[{track_id}] Updating duration from {track.duration} to {accurate_duration}."
            )
            track.duration = accurate_duration
            track_updated = True

        if cover_data:
            logger.info(f"[{track_id}] Found cover data, processing and saving.")
            new_has_cover = await cover_processor.process_and_save_cover(
                track_id, cover_data, file_path
            )
            logger.info(
                f"[{track_id}] Cover processing finished. Has cover: {new_has_cover}."
            )
        else:
            logger.info(f"[{track_id}] No cover data found in file.")
            new_has_cover = False

        if track.has_cover != new_has_cover:
            logger.info(
                f"[{track_id}] Updating has_cover from {track.has_cover} to {new_has_cover}."
            )
            track.has_cover = new_has_cover
            track_updated = True

        if track_updated:
            logger.info(f"[{track_id}] Track has updates, saving to database.")
            track.updated_at = int(time.time())
            await update_track(track)
            logger.info(f"[{track_id}] Track successfully updated in database.")
        else:
            logger.info(f"[{track_id}] No metadata changes for track.")

        logger.info(f"[{track_id}] Finished slow metadata processing successfully.")
        return track
    except Exception as e:
        logger.error(
            f"[{track_id}] Unhandled exception in process_slow_metadata_for_track: {e}",
            exc_info=True,
        )
        raise


def _needs_id3_standardization(audio: MutagenFile) -> bool:
    if hasattr(audio, "tags") and audio.tags:
        if hasattr(audio.tags, "version") and audio.tags.version == (2, 3, 0):
            return False
        return True
    return False
