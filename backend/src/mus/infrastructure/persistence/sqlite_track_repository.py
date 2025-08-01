from typing import Optional, Sequence
from sqlmodel import select, desc
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.dialects.sqlite import insert as sqlite_upsert
from sqlalchemy.engine import Row

from src.mus.domain.entities.track import Track


class SQLiteTrackRepository:
    def __init__(self, session: AsyncSession):
        self.session = session

    async def get_by_id(self, track_id: Optional[int]) -> Optional[Track]:
        if track_id is None:
            return None
        result = await self.session.exec(select(Track).where(Track.id == track_id))
        return result.first()

    async def get_all(self) -> Sequence[Row]:
        result = await self.session.execute(
            select(
                Track.id,
                Track.title,
                Track.artist,
                Track.duration,
                Track.file_path,
                Track.updated_at,
                Track.has_cover,
            ).order_by(desc(Track.added_at))
        )
        return result.all()

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
            updated_at=track_data.updated_at,
            inode=track_data.inode,
            content_hash=track_data.content_hash,
            processing_status=track_data.processing_status,
            last_error=track_data.last_error,
        )
        stmt = stmt.on_conflict_do_update(
            index_elements=["file_path"],
            set_={
                "title": stmt.excluded.title,
                "artist": stmt.excluded.artist,
                "duration": stmt.excluded.duration,
                "has_cover": stmt.excluded.has_cover,
                "updated_at": stmt.excluded.updated_at,
                "inode": stmt.excluded.inode,
                "content_hash": stmt.excluded.content_hash,
                "processing_status": stmt.excluded.processing_status,
                "last_error": stmt.excluded.last_error,
            },
        )
        await self.session.execute(stmt)

        result = await self.session.exec(
            select(Track).where(Track.file_path == track_data.file_path)
        )
        return result.one()

    async def get_by_inode(self, inode: int) -> Optional[Track]:
        result = await self.session.exec(select(Track).where(Track.inode == inode))
        return result.first()
