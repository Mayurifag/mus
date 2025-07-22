import logging
from pathlib import Path
from typing import Set

from watchfiles import awatch, Change

from src.mus.config import settings
from src.mus.core.arq_pool import get_arq_pool

logger = logging.getLogger(__name__)

AUDIO_EXTENSIONS = {".mp3", ".flac", ".m4a", ".ogg", ".wav"}


async def watch_music_directory():
    """
    Watch the music directory for file system changes and enqueue appropriate jobs.
    Uses watchfiles.awatch for asynchronous file monitoring.
    """
    music_dir = Path(settings.MUSIC_DIR_PATH)

    if not music_dir.exists():
        logger.warning(f"Music directory does not exist: {music_dir}")
        return

    logger.info(f"Starting file watcher for: {music_dir}")
    arq_pool = await get_arq_pool()

    try:
        async for changes in awatch(str(music_dir)):
            await _process_file_changes(changes, arq_pool)
    except Exception as e:
        logger.error(f"File watcher error: {e}")
        raise


async def _process_file_changes(changes: Set, arq_pool):
    """Process a set of file system changes."""
    for change_type, file_path_str in changes:
        file_path = Path(file_path_str)

        # Only process audio files
        if not _is_audio_file(file_path):
            continue

        try:
            if change_type == Change.added:
                await arq_pool.enqueue_job(
                    "handle_file_created",
                    file_path_str=file_path_str,
                )
                logger.debug(f"Enqueued file_created job for: {file_path}")

            elif change_type == Change.modified:
                await arq_pool.enqueue_job(
                    "handle_file_modified",
                    file_path_str=file_path_str,
                )
                logger.debug(f"Enqueued file_modified job for: {file_path}")

            elif change_type == Change.deleted:
                await arq_pool.enqueue_job(
                    "handle_file_deleted",
                    file_path_str=file_path_str,
                )
                logger.debug(f"Enqueued file_deleted job for: {file_path}")

        except Exception as e:
            logger.error(f"Error enqueuing job for {file_path}: {e}")


def _is_audio_file(file_path: Path) -> bool:
    """Check if the file is an audio file based on its extension."""
    return file_path.suffix.lower() in AUDIO_EXTENSIONS
