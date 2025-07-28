import logging
from pathlib import Path

from src.mus.config import settings

logger = logging.getLogger(__name__)


class PermissionsService:
    def __init__(self):
        self.can_write_music_files: bool = False

    def check_permissions(self):
        music_dir = Path(settings.MUSIC_DIR_PATH)
        try:
            music_dir.mkdir(parents=True, exist_ok=True)
            test_file = music_dir / ".write_test"
            test_file.write_text("test")
            test_file.unlink()
            self.can_write_music_files = True
        except (OSError, PermissionError) as e:
            self.can_write_music_files = False
            logger.warning(f"No write permissions for {music_dir}: {e}")

        essential_directories = [
            Path(settings.COVERS_DIR_PATH),
            Path(settings.DATABASE_PATH).parent,
        ]

        for directory in essential_directories:
            if directory is None:
                continue
            try:
                directory.mkdir(parents=True, exist_ok=True)
                test_file = directory / ".write_test"
                test_file.write_text("test")
                test_file.unlink()
            except (OSError, PermissionError) as e:
                logger.error(f"No write permissions for {directory}: {e}")
                raise RuntimeError(f"Cannot write to {directory}: {e}")
