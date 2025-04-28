from abc import ABC, abstractmethod
from collections.abc import AsyncGenerator
from pathlib import Path


class IFileScanner(ABC):
    @abstractmethod
    async def find_music_files(self) -> AsyncGenerator[Path, None]:
        """Find all music files in the configured directory."""
        pass
