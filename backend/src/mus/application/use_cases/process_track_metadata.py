import asyncio
import os
import time
from pathlib import Path
from typing import Optional

from mutagen._file import File as MutagenFile

from src.mus.core.redis import set_app_write_lock
from src.mus.config import settings
from src.mus.domain.entities.track import Track
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.util.ffprobe_analyzer import get_accurate_duration
from src.mus.util.db_utils import get_track_by_id, update_track


async def process_slow_metadata_for_track(track_id: int) -> Optional[Track]:
    track = await get_track_by_id(track_id)
    if not track or not Path(track.file_path).exists():
        return None

    file_path = Path(track.file_path)

    original_mtime = file_path.stat().st_mtime

    audio = MutagenFile(file_path, easy=False)
    if audio and _needs_id3_standardization(audio):
        await set_app_write_lock(str(file_path))
        audio.save(v2_version=3)
        os.utime(file_path, (original_mtime, original_mtime))

    cover_processor = CoverProcessor(settings.COVERS_DIR_PATH)

    duration_task = asyncio.to_thread(get_accurate_duration, file_path)
    cover_task = cover_processor.extract_cover_from_file(file_path)

    accurate_duration, cover_data = await asyncio.gather(duration_task, cover_task)

    track_updated = False

    if accurate_duration > 0 and accurate_duration != track.duration:
        track.duration = accurate_duration
        track_updated = True

    new_has_cover = bool(
        cover_data
        and await cover_processor.process_and_save_cover(
            track_id, cover_data, file_path
        )
    )

    if track.has_cover != new_has_cover:
        track.has_cover = new_has_cover
        track_updated = True

    if track_updated:
        track.updated_at = int(time.time())
        await update_track(track)

    return track


def _needs_id3_standardization(audio: MutagenFile) -> bool:
    if hasattr(audio, "tags") and audio.tags:
        if hasattr(audio.tags, "version") and audio.tags.version == (2, 3, 0):
            return False
        return True
    return False
