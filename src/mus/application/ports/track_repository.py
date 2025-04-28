from pathlib import Path
from typing import Protocol

from mus.domain.track import Track


class ITrackRepository(Protocol):
    async def add(self, track: Track) -> None: ...

    async def exists_by_path(self, file_path: Path) -> bool: ...

    async def search_by_title(self, query: str) -> list[Track]: ...

    async def get_all(self) -> list[Track]: ...
