from collections.abc import AsyncGenerator
from pathlib import Path
from typing import ClassVar

from mus.domain.scanners.file_scanner import IFileScanner


class FileSystemScanner(IFileScanner):
    SUPPORTED_EXTENSIONS: ClassVar[set[str]] = {".mp3", ".flac", ".ogg", ".m4a", ".wav"}

    def __init__(self, root_dir: Path):
        self.root_dir = root_dir

    async def find_music_files(self) -> AsyncGenerator[Path, None]:
        async def scan_directory(directory: Path):
            try:
                for item in directory.iterdir():
                    if (
                        item.is_file()
                        and item.suffix.lower() in self.SUPPORTED_EXTENSIONS
                    ):
                        yield item
                    elif item.is_dir():
                        async for file in scan_directory(item):
                            yield file
            except PermissionError:
                # Skip directories we don't have permission to access
                pass

        async for file in scan_directory(self.root_dir):
            yield file
