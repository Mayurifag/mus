import logging
from pathlib import Path
from typing import Set

from watchfiles import Change, DefaultFilter, awatch

from src.mus.config import settings
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.jobs.file_system_jobs import (
    handle_file_created,
    handle_file_deleted,
    handle_file_modified,
)

logger = logging.getLogger(__name__)

AUDIO_EXTENSIONS = {".mp3", ".flac", ".m4a", ".ogg", ".wav"}


class MusicDirectoryFilter(DefaultFilter):
    """Custom filter that ignores changes in the music directory itself."""

    def __call__(self, change: Change, path: str) -> bool:
        # First apply the default filter logic
        if not super().__call__(change, path):
            return False

        # Ignore changes to the music directory itself (not its contents)
        music_dir_path = str(settings.MUSIC_DIR_PATH)
        if path == music_dir_path:
            return False

        return True


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
        async for changes in awatch(
            str(music_dir), watch_filter=MusicDirectoryFilter()
        ):
            await _process_file_changes(changes)
    except Exception as e:
        logger.error(f"File watcher error: {e}")
        raise


async def _process_file_changes(changes: Set):
    """Process a set of file system changes."""
    audio_changes = []

    for change_type, file_path_str in changes:
        file_path = Path(file_path_str)

        # Only process audio files
        if not _is_audio_file(file_path):
            continue

        audio_changes.append((change_type, file_path_str, file_path))

    if not audio_changes:
        return

    for change_type, file_path_str, file_path in audio_changes:
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
