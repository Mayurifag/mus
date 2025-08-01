from contextlib import asynccontextmanager
from typing import List, Optional

from sqlalchemy.dialects.sqlite import insert as sqlite_upsert
from sqlmodel import col, select

from src.mus.domain.entities.track import ProcessingStatus, Track
from src.mus.infrastructure.database import async_session_factory
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


@asynccontextmanager
async def get_track_repo():
    async with async_session_factory() as session:
        yield SQLiteTrackRepository(session), session


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


async def delete_track_from_db_by_path(file_path: str) -> Optional[int]:
    async with get_track_repo() as (_, session):
        result = await session.exec(select(Track).where(Track.file_path == file_path))
        track = result.first()
        if track:
            track_id = track.id
            await session.delete(track)
            await session.commit()
            return track_id
        return None


async def delete_track_from_db_by_id(track_id: int) -> bool:
    async with get_track_repo() as (_, session):
        result = await session.exec(select(Track).where(Track.id == track_id))
        track = result.first()
        if track:
            await session.delete(track)
            await session.commit()
            return True
        return False


async def update_track_path(track_id: int, dest_path: str) -> Optional[Track]:
    async with get_track_repo() as (_, session):
        result = await session.exec(select(Track).where(Track.id == track_id))
        track = result.first()
        if track:
            track.file_path = dest_path
            await session.commit()
            await session.refresh(track)
            return track
        return None


async def get_tracks_by_status(status: ProcessingStatus) -> List[Track]:
    async with get_track_repo() as (_, session):
        result = await session.exec(
            select(Track).where(Track.processing_status == status)
        )
        return list(result.all())


async def upsert_tracks_batch(tracks: List[Track]) -> List[Track]:
    if not tracks:
        return []

    async with get_track_repo() as (_, session):
        # Prepare track data for bulk insert
        track_values = [track.model_dump(exclude={"id"}) for track in tracks]

        # Create bulk upsert statement
        stmt = sqlite_upsert(Track).values(track_values)
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
        ).returning(
            sqlite_upsert(Track).table.c.id, sqlite_upsert(Track).table.c.file_path
        )

        # Execute bulk operation
        result = await session.execute(stmt)
        upserted_data = result.fetchall()
        await session.commit()

        # Fetch the complete track objects for the upserted tracks
        if upserted_data:
            track_ids = [row[0] for row in upserted_data]
            result = await session.exec(
                select(Track).where(col(Track.id).in_(track_ids))
            )
            return list(result.all())

        return []
