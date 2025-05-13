from typing import List, Optional
from sqlmodel import select, desc
from sqlmodel.ext.asyncio.session import AsyncSession
from fastapi import Depends
from sqlalchemy.dialects.sqlite import insert as sqlite_upsert

from src.mus.domain.entities.track import Track
from src.mus.infrastructure.database import get_session_generator


class SQLiteTrackRepository:
    def __init__(self, session: AsyncSession = Depends(get_session_generator)):
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
            await self.session.commit()

    async def upsert_track(self, track_data: Track) -> Track:
        track_dict = {
            "title": track_data.title,
            "artist": track_data.artist,
            "duration": track_data.duration,
            "file_path": track_data.file_path,
            "has_cover": track_data.has_cover,
        }

        if track_data.id is not None:
            track_dict["id"] = track_data.id

        stmt = sqlite_upsert(Track).values(**track_dict)

        stmt = stmt.on_conflict_do_update(
            index_elements=["file_path"],
            set_={
                "title": stmt.excluded.title,
                "artist": stmt.excluded.artist,
                "duration": stmt.excluded.duration,
                "has_cover": stmt.excluded.has_cover,
            },
        )

        await self.session.execute(stmt)
        await self.session.commit()

        result = await self.session.exec(
            select(Track).where(Track.file_path == track_data.file_path)
        )
        return result.one()
