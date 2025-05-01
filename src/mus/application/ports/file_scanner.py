from collections.abc import AsyncIterator
from pathlib import Path
from typing import Protocol


class IFileScanner(Protocol):
    async def find_music_files(self, directory: Path) -> AsyncIterator[Path]:
        """Find all music files in a directory recursively.

        Args:
            directory: The directory to scan.

        Returns:
            AsyncIterator[Path]: An async iterator of music file paths.
        """
        ...
