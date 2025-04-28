from collections.abc import AsyncIterator
from pathlib import Path
from typing import Protocol


class IFileScanner(Protocol):
    async def find_music_files(self, directory: Path) -> AsyncIterator[Path]: ...
