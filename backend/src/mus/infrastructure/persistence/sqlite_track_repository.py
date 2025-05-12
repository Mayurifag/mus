from typing import List, Optional
from sqlmodel import select, desc
from sqlmodel.ext.asyncio.session import AsyncSession
from fastapi import Depends
from sqlalchemy.dialects.sqlite import insert as sqlite_upsert

from src.mus.domain.entities.track import Track
from src.mus.infrastructure.database import get_session_generator


class SQLiteTrackRepository:
    """Repository for Track entities in SQLite database."""

    def __init__(self, session: AsyncSession = Depends(get_session_generator)):
        """Initialize with a session from dependency injection."""
        self.session = session

    async def get_by_id(self, track_id: Optional[int]) -> Optional[Track]:
        """Get a track by ID, return None if no ID provided."""
        if track_id is None:
            return None
        result = await self.session.exec(select(Track).where(Track.id == track_id))
        return result.first()

    async def get_all(self) -> List[Track]:
        """Get all tracks ordered by added_at timestamp (newest first)."""
        result = await self.session.exec(select(Track).order_by(desc(Track.added_at)))
        return list(result.all())

    async def exists_by_path(self, file_path: str) -> bool:
        """Check if a track exists by file path."""
        result = await self.session.exec(
            select(Track).where(Track.file_path == file_path)
        )
        return result.first() is not None

    async def add(self, track: Track) -> Track:
        """Add a new track to the database."""
        self.session.add(track)
        await self.session.commit()
        await self.session.refresh(track)
        return track

    async def set_cover_flag(self, track_id: Optional[int], has_cover: bool) -> None:
        """Update the has_cover flag for a track, do nothing if no ID."""
        if track_id is None:
            return
        track = await self.session.get(Track, track_id)
        if track:
            track.has_cover = has_cover
            await self.session.commit()

    async def upsert_track(self, track_data: Track) -> Track:
        """
        Insert a track or update if it already exists by file_path.

        The id and added_at fields of existing records are preserved.
        Other fields (title, artist, duration, and has_cover) are updated.

        Returns the final state of the track from the database.
        """
        # Convert track_data to a dictionary for the UPSERT operation
        track_dict = {
            "title": track_data.title,
            "artist": track_data.artist,
            "duration": track_data.duration,
            "file_path": track_data.file_path,
            "has_cover": track_data.has_cover,
        }

        # If track_data has an ID, include it in the dictionary
        if track_data.id is not None:
            track_dict["id"] = track_data.id

        # Create upsert statement
        stmt = sqlite_upsert(Track).values(**track_dict)

        # Configure on_conflict_do_update to update all fields except id and added_at
        stmt = stmt.on_conflict_do_update(
            index_elements=["file_path"],
            set_={
                "title": stmt.excluded.title,
                "artist": stmt.excluded.artist,
                "duration": stmt.excluded.duration,
                "has_cover": stmt.excluded.has_cover,
            },
        )

        # Execute the statement
        # Note: Using execute instead of exec because SQLModel's exec doesn't support dialect-specific insert
        await self.session.execute(stmt)
        await self.session.commit()

        # Retrieve the updated or inserted track
        result = await self.session.exec(
            select(Track).where(Track.file_path == track_data.file_path)
        )
        return result.one()
