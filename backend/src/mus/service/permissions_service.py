from typing import Optional

from src.mus.config import settings


class PermissionsService:
    def __init__(self):
        self._is_writable: Optional[bool] = None

    def check_write_permissions(self) -> bool:
        if self._is_writable is not None:
            return self._is_writable

        try:
            test_file = settings.MUSIC_DIR_PATH / ".writable_check"
            test_file.touch()
            test_file.unlink()
            self._is_writable = True
        except OSError:
            self._is_writable = False

        return self._is_writable
