from pathlib import Path

from watchdog.events import FileSystemEvent, FileSystemEventHandler
from watchdog.observers import Observer

from src.mus.util.queue_utils import (
    enqueue_file_created,
    enqueue_file_modified,
    enqueue_file_deletion,
    enqueue_file_move,
)

AUDIO_EXTENSIONS = {".mp3", ".flac", ".m4a", ".ogg", ".wav"}


class MusicFileEventHandler(FileSystemEventHandler):
    def _is_audio_file(self, file_path: str) -> bool:
        return Path(file_path).suffix.lower() in AUDIO_EXTENSIONS

    def on_created(self, event: FileSystemEvent):
        if not event.is_directory and self._is_audio_file(str(event.src_path)):
            enqueue_file_created(str(event.src_path))

    def on_modified(self, event: FileSystemEvent):
        if not event.is_directory and self._is_audio_file(str(event.src_path)):
            enqueue_file_modified(str(event.src_path))

    def on_deleted(self, event: FileSystemEvent):
        if not event.is_directory and self._is_audio_file(str(event.src_path)):
            enqueue_file_deletion(str(event.src_path))

    def on_moved(self, event: FileSystemEvent):
        if not event.is_directory and self._is_audio_file(str(event.src_path)):
            enqueue_file_move(str(event.src_path), str(event.dest_path))


class FileWatcherService:
    def __init__(self, music_dir: Path):
        self.music_dir = music_dir
        self.observer = Observer()
        self.event_handler = MusicFileEventHandler()

    def start(self):
        self.observer.schedule(self.event_handler, str(self.music_dir), recursive=True)
        self.observer.start()

    def stop(self):
        self.observer.stop()
        self.observer.join()
