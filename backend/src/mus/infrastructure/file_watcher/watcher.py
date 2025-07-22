import logging
from pathlib import Path
from typing import Set

from watchfiles import awatch, Change

from src.mus.config import settings
from src.mus.core.streaq_broker import worker

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

    try:
        async for changes in awatch(str(music_dir)):
            await _process_file_changes(changes)
    except Exception as e:
        logger.error(f"File watcher error: {e}")
        raise


async def _process_file_changes(changes: Set):
    """Process a set of file system changes."""
    from src.mus.infrastructure.jobs.file_system_jobs import (
        handle_file_created,
        handle_file_modified,
        handle_file_deleted,
    )

    logger.info(f"Processing {len(changes)} file changes")
    for change_type, file_path_str in changes:
        file_path = Path(file_path_str)
        logger.info(f"Change detected: {change_type} for {file_path}")

        # Only process audio files
        if not _is_audio_file(file_path):
            logger.debug(f"Skipping non-audio file: {file_path}")
            continue

        try:
            async with worker:
                if change_type == Change.added:
                    await handle_file_created.enqueue(
                        file_path_str=file_path_str,
                    )
                    logger.info(f"Enqueued file_created job for: {file_path}")

                elif change_type == Change.modified:
                    await handle_file_modified.enqueue(
                        file_path_str=file_path_str,
                    )
                    logger.info(f"Enqueued file_modified job for: {file_path}")

                elif change_type == Change.deleted:
                    await handle_file_deleted.enqueue(
                        file_path_str=file_path_str,
                    )
                    logger.info(f"Enqueued file_deleted job for: {file_path}")

        except Exception as e:
            logger.error(f"Error enqueuing job for {file_path}: {e}")


def _is_audio_file(file_path: Path) -> bool:
    """Check if the file is an audio file based on its extension."""
    return file_path.suffix.lower() in AUDIO_EXTENSIONS
