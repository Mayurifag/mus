import os
import asyncio
from collections.abc import AsyncGenerator
from pathlib import Path
from typing import ClassVar, List, Optional, Set


class FileSystemScanner:
    SUPPORTED_EXTENSIONS: ClassVar[set[str]] = {".mp3", ".flac"}

    # Hardcoded music directory path
    MUSIC_DIR = "./music"

    def __init__(self):
        # Create music directory if it doesn't exist
        os.makedirs(self.MUSIC_DIR, exist_ok=True)
        self.root_dir = Path(self.MUSIC_DIR)

    async def find_music_files(
        self, directory: Path, extensions: Optional[Set[str]] = None
    ) -> AsyncGenerator[Path, None]:
        if not directory.exists():
            return

        extensions = extensions or self.SUPPORTED_EXTENSIONS

        try:
            dir_contents = list(directory.iterdir())
        except (PermissionError, OSError):
            # Log error if needed
            return

        # Process files first
        for item in dir_contents:
            if item.is_file() and item.suffix.lower() in extensions:
                yield item

        # Then process subdirectories concurrently
        subdirs = [item for item in dir_contents if item.is_dir()]

        # Process subdirectories in chunks to avoid creating too many tasks
        chunk_size = 5
        for i in range(0, len(subdirs), chunk_size):
            subdirs_chunk = subdirs[i : i + chunk_size]
            tasks = [
                self._scan_directory(subdir, extensions) for subdir in subdirs_chunk
            ]
            results = await asyncio.gather(*tasks, return_exceptions=True)

            for result in results:
                if not isinstance(result, list):
                    # Skip if the result is an exception
                    continue

                for file_path in result:
                    yield file_path

    async def _scan_directory(
        self, directory: Path, extensions: Set[str]
    ) -> List[Path]:
        """Helper method to scan a directory and return all matching files."""
        result = []
        try:
            stack = [directory]
            while stack:
                current_dir = stack.pop()
                try:
                    for item in current_dir.iterdir():
                        if item.is_file() and item.suffix.lower() in extensions:
                            result.append(item)
                        elif item.is_dir():
                            stack.append(item)
                except (PermissionError, OSError):
                    continue
        except Exception:
            # Catch any other exceptions to ensure task doesn't fail
            pass

        return result

    async def scan_directories(
        self,
        directories: Optional[List[Path]] = None,
        extensions: Optional[Set[str]] = None,
    ) -> AsyncGenerator[Path, None]:
        """
        Scan multiple directories concurrently for music files

        Args:
            directories: List of directories to scan, defaults to the music directory
            extensions: Optional set of file extensions to look for
        """
        scan_dirs = directories or [self.root_dir]

        # Process directories concurrently in batches to avoid overwhelming the system
        for directory in scan_dirs:
            async for file_path in self.find_music_files(directory, extensions):
                yield file_path
