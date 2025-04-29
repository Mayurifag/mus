from abc import ABC, abstractmethod
from pathlib import Path

from mus.domain.track import Track


class ITrackRepository(ABC):
    @abstractmethod
    async def add(self, track: Track) -> None:
        """Add a track to the repository."""
        pass

    @abstractmethod
    async def exists_by_path(self, file_path: Path) -> bool:
        """Check if a track with the given file path exists."""
        pass

    @abstractmethod
    async def search_by_title(self, query: str) -> list[Track]:
        """Search for tracks by title."""
        pass

    @abstractmethod
    async def get_all(self) -> list[Track]:
        """Get all tracks."""
        pass

    @abstractmethod
    async def clear_all_tracks(self) -> None:
        """Clear all tracks from the repository."""
        pass
