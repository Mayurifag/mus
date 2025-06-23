from typing import List, Optional
from sqlmodel import select, desc
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.track_history import TrackHistory


class SQLiteTrackHistoryRepository:
    def __init__(self, session: AsyncSession):
        self.session = session

    async def add(self, history_entry: TrackHistory) -> TrackHistory:
        self.session.add(history_entry)
        await self.session.commit()
        await self.session.refresh(history_entry)
        return history_entry

    async def get_by_track_id(self, track_id: int) -> List[TrackHistory]:
        result = await self.session.exec(
            select(TrackHistory)
            .where(TrackHistory.track_id == track_id)
            .order_by(desc(TrackHistory.changed_at))
        )
        return list(result.all())

    async def get_by_id(self, history_id: int) -> Optional[TrackHistory]:
        result = await self.session.exec(
            select(TrackHistory).where(TrackHistory.id == history_id)
        )
        return result.first()

    async def prune_history(self, track_id: int, keep: int = 5):
        result = await self.session.exec(
            select(TrackHistory)
            .where(TrackHistory.track_id == track_id)
            .order_by(desc(TrackHistory.changed_at))
            .offset(keep)
        )
        old_entries = list(result.all())
        for entry in old_entries:
            await self.session.delete(entry)
        await self.session.commit()
