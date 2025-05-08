from abc import abstractmethod
from pathlib import Path
from typing import Protocol

from mus.domain.track import Track


class ITrackRepository(Protocol):
    @abstractmethod
    async def initialize_schema(self) -> None:
        """Initialize the database schema, creating tables and indices."""
        pass

    @abstractmethod
    async def add(self, track: Track) -> int:
        """Add a track to the repository.

        Args:
            track: The track to add.

        Returns:
            int: The ID of the newly added track.
        """
        pass

    @abstractmethod
    async def exists_by_path(self, file_path: Path) -> bool:
        """Check if a track exists by its file path.

        Args:
            file_path: The path to check.

        Returns:
            bool: True if the track exists, False otherwise.
        """
        pass

    @abstractmethod
    async def search_by_title(self, query: str) -> list[Track]:
        """Search for tracks by title.

        Args:
            query: The search query.

        Returns:
            list[Track]: A list of matching tracks.
        """
        pass

    @abstractmethod
    async def get_all(self) -> list[Track]:
        """Get all tracks in the repository.

        Returns:
            list[Track]: A list of all tracks.
        """
        pass

    @abstractmethod
    async def clear_all_tracks(self) -> None:
        """Clear all tracks from the repository."""
        pass

    @abstractmethod
    async def get_by_id(self, track_id: int) -> Track | None:
        """Get a track by its ID.

        Args:
            track_id: The ID of the track to get.

        Returns:
            Track | None: The track if found, None otherwise.
        """
        pass

    @abstractmethod
    async def set_cover_flag(self, track_id: int, has_cover: bool) -> None:
        """Set the has_cover flag for a track.

        Args:
            track_id: The ID of the track to update.
            has_cover: Whether the track has a cover image.
        """
        pass
