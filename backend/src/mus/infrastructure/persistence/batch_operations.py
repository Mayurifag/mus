from typing import List

from sqlalchemy.dialects.sqlite import insert as sqlite_upsert
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.track import Track
from src.mus.util.queue_utils import enqueue_slow_metadata


async def batch_upsert_tracks(session: AsyncSession, tracks: List[Track]) -> int:
    if not tracks:
        return 0

    track_values = [track.model_dump(exclude={"id"}) for track in tracks]

    stmt = sqlite_upsert(Track).values(track_values)
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
    ).returning(sqlite_upsert(Track).table.c.id)

    result = await session.execute(stmt)
    track_ids = result.scalars().all()
    await session.commit()

    if track_ids:
        for track_id in track_ids:
            enqueue_slow_metadata(track_id)

    return len(track_ids)
