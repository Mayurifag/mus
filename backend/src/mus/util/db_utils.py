from contextlib import asynccontextmanager
from typing import Optional
from sqlmodel import select

from src.mus.infrastructure.database import async_session_factory
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.persistence.sqlite_track_history_repository import (
    SQLiteTrackHistoryRepository,
)
from src.mus.domain.entities.track import Track
from src.mus.domain.entities.track_history import TrackHistory


@asynccontextmanager
async def get_track_repo():
    async with async_session_factory() as session:
        yield SQLiteTrackRepository(session), session


@asynccontextmanager
async def get_track_history_repo():
    async with async_session_factory() as session:
        yield SQLiteTrackHistoryRepository(session), session


async def get_track_by_id(track_id: int) -> Optional[Track]:
    async with get_track_repo() as (repo, _):
        return await repo.get_by_id(track_id)


async def get_track_by_path(file_path: str) -> Optional[Track]:
    async with get_track_repo() as (_, session):
        result = await session.exec(select(Track).where(Track.file_path == file_path))
        return result.first()


async def get_track_by_inode(inode: int) -> Optional[Track]:
    async with get_track_repo() as (repo, _):
        return await repo.get_by_inode(inode)


async def upsert_track(track: Track) -> Track:
    async with get_track_repo() as (repo, session):
        result = await repo.upsert_track(track)
        await session.commit()
        return result


async def update_track(track: Track):
    async with get_track_repo() as (_, session):
        session.add(track)
        await session.commit()


async def delete_track_by_path(file_path: str) -> Optional[int]:
    async with get_track_repo() as (_, session):
        result = await session.exec(select(Track).where(Track.file_path == file_path))
        track = result.first()
        if track:
            track_id = track.id
            await session.delete(track)
            await session.commit()
            return track_id
        return None


async def update_track_path(src_path: str, dest_path: str) -> bool:
    async with get_track_repo() as (_, session):
        result = await session.exec(select(Track).where(Track.file_path == src_path))
        track = result.first()
        if track:
            track.file_path = dest_path
            await session.commit()
            return True
        return False


async def add_track_history(history_entry: TrackHistory) -> TrackHistory:
    async with get_track_history_repo() as (repo, _):
        return await repo.add(history_entry)


async def prune_track_history(track_id: int, keep: int = 5):
    async with get_track_history_repo() as (repo, _):
        await repo.prune_history(track_id, keep)
