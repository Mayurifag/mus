from abc import ABC, abstractmethod
from collections.abc import AsyncGenerator
from pathlib import Path


class IFileScanner(ABC):
    @abstractmethod
    async def find_music_files(self, directory: Path) -> AsyncGenerator[Path, None]:
        """Find all music files in a directory recursively.

        Args:
            directory: The directory to scan.

        Returns:
            AsyncGenerator[Path, None]: An async generator of music file paths.
        """
        pass
