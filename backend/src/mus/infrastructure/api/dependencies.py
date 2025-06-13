from fastapi import Depends
from sqlmodel.ext.asyncio.session import AsyncSession
from src.mus.infrastructure.database import get_session_generator
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


async def get_track_repository(
    session: AsyncSession = Depends(get_session_generator),
) -> SQLiteTrackRepository:
    return SQLiteTrackRepository(session)
