from typing import List, Optional
from sqlmodel import select, desc, func
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.dialects.sqlite import insert as sqlite_upsert

from src.mus.domain.entities.track import Track


class SQLiteTrackRepository:
    def __init__(self, session: AsyncSession):
        self.session = session

    async def get_by_id(self, track_id: Optional[int]) -> Optional[Track]:
        if track_id is None:
            return None
        result = await self.session.exec(select(Track).where(Track.id == track_id))
        return result.first()

    async def get_all(self) -> List[Track]:
        result = await self.session.exec(select(Track).order_by(desc(Track.added_at)))
        return list(result.all())

    async def exists_by_path(self, file_path: str) -> bool:
        result = await self.session.exec(
            select(Track).where(Track.file_path == file_path)
        )
        return result.first() is not None

    async def add(self, track: Track) -> Track:
        self.session.add(track)
        await self.session.commit()
        await self.session.refresh(track)
        return track

    async def set_cover_flag(self, track_id: Optional[int], has_cover: bool) -> None:
        if track_id is None:
            return
        track = await self.session.get(Track, track_id)
        if track:
            track.has_cover = has_cover

    async def upsert_track(self, track_data: Track) -> Track:
        stmt = sqlite_upsert(Track).values(
            title=track_data.title,
            artist=track_data.artist,
            duration=track_data.duration,
            file_path=track_data.file_path,
            has_cover=track_data.has_cover,
            added_at=track_data.added_at,
            inode=track_data.inode,
            content_hash=track_data.content_hash,
            processing_status=track_data.processing_status,
            last_error_message=track_data.last_error_message,
        )
        stmt = stmt.on_conflict_do_update(
            index_elements=["file_path"],
            set_={
                "title": stmt.excluded.title,
                "artist": stmt.excluded.artist,
                "duration": stmt.excluded.duration,
                "has_cover": stmt.excluded.has_cover,
                "inode": stmt.excluded.inode,
                "content_hash": stmt.excluded.content_hash,
                "processing_status": stmt.excluded.processing_status,
                "last_error_message": stmt.excluded.last_error_message,
            },
        )
        await self.session.execute(stmt)

        result = await self.session.exec(
            select(Track).where(Track.file_path == track_data.file_path)
        )
        return result.one()

    async def get_latest_track_added_at(self) -> Optional[int]:
        """Fetches the maximum 'added_at' timestamp from all tracks."""
        result = await self.session.exec(select(func.max(Track.added_at)))
        latest_added_at = result.one_or_none()
        return latest_added_at

    async def get_by_inode(self, inode: int) -> Optional[Track]:
        result = await self.session.exec(select(Track).where(Track.inode == inode))
        return result.first()

    async def delete_by_path(self, file_path: str) -> bool:
        result = await self.session.exec(
            select(Track).where(Track.file_path == file_path)
        )
        track = result.first()
        if track:
            await self.session.delete(track)
            return True
        return False
