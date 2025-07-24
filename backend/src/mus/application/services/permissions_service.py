import logging
from pathlib import Path

from src.mus.config import settings

logger = logging.getLogger(__name__)


class PermissionsService:
    def check_write_permissions(self):
        directories_to_check = [
            Path(settings.MUSIC_DIR_PATH),
            Path(settings.COVERS_DIR_PATH),
        ]

        for directory in directories_to_check:
            try:
                directory.mkdir(parents=True, exist_ok=True)

                test_file = directory / ".write_test"
                test_file.write_text("test")
                test_file.unlink()

                logger.info(f"Write permissions OK for: {directory}")

            except (OSError, PermissionError) as e:
                logger.error(f"No write permissions for {directory}: {e}")
                raise RuntimeError(f"Cannot write to {directory}: {e}")
