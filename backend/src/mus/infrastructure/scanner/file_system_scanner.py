import os
from collections.abc import AsyncGenerator
from pathlib import Path
from typing import ClassVar


class FileSystemScanner:
    SUPPORTED_EXTENSIONS: ClassVar[set[str]] = {".mp3", ".flac"}

    # Hardcoded music directory path
    MUSIC_DIR = "./music"

    def __init__(self):
        # Create music directory if it doesn't exist
        os.makedirs(self.MUSIC_DIR, exist_ok=True)
        self.root_dir = Path(self.MUSIC_DIR)

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
