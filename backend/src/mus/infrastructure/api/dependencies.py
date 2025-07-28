from fastapi import Depends
from sqlmodel.ext.asyncio.session import AsyncSession
from src.mus.infrastructure.database import get_session_generator
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.api.permissions_singleton import permissions_service
from src.mus.application.services.permissions_service import PermissionsService


async def get_track_repository(
    session: AsyncSession = Depends(get_session_generator),
) -> SQLiteTrackRepository:
    return SQLiteTrackRepository(session)


def get_permissions_service() -> PermissionsService:
    return permissions_service
