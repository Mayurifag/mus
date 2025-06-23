from src.mus.config import settings
from src.mus.service.file_watcher_service import FileWatcherService


class FileWatcherManager:
    def __init__(self):
        self.file_watcher = None

    async def start(self) -> None:
        self.file_watcher = FileWatcherService(settings.MUSIC_DIR_PATH)
        self.file_watcher.start()

    def stop(self) -> None:
        if self.file_watcher:
            self.file_watcher.stop()
