from collections.abc import AsyncGenerator
from pathlib import Path
from typing import ClassVar

from mus.application.ports.file_scanner import IFileScanner


class FileSystemScanner(IFileScanner):
    SUPPORTED_EXTENSIONS: ClassVar[set[str]] = {".mp3", ".flac", ".ogg", ".m4a", ".wav"}

    def __init__(self, root_dir: Path):
        self.root_dir = root_dir

    async def find_music_files(self, directory: Path) -> AsyncGenerator[Path, None]:
        stack = [directory]
        while stack:
            current_dir = stack.pop()
            try:
                for item in current_dir.iterdir():
                    if (
                        item.is_file()
                        and item.suffix.lower() in self.SUPPORTED_EXTENSIONS
                    ):
                        yield item
                    elif item.is_dir():
                        stack.append(item)
            except PermissionError:
                continue
