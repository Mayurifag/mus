from typing import List
from fastapi import APIRouter, Depends, HTTPException
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.application.dtos.track_history import TrackHistoryDTO
from src.mus.infrastructure.database import get_session_generator
from src.mus.infrastructure.persistence.sqlite_track_history_repository import (
    SQLiteTrackHistoryRepository,
)
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker


router = APIRouter(prefix="/api/v1/tracks", tags=["history"])


@router.get("/{track_id}/history", response_model=List[TrackHistoryDTO])
async def get_track_history(
    track_id: int,
    session: AsyncSession = Depends(get_session_generator),
) -> List[TrackHistoryDTO]:
    history_repo = SQLiteTrackHistoryRepository(session)
    history_entries = await history_repo.get_by_track_id(track_id)
    return [TrackHistoryDTO.model_validate(entry) for entry in history_entries]


@router.post("/history/{history_id}/rollback")
async def rollback_track_history(
    history_id: int,
    session: AsyncSession = Depends(get_session_generator),
):
    history_repo = SQLiteTrackHistoryRepository(session)
    track_repo = SQLiteTrackRepository(session)

    history_entry = await history_repo.get_by_id(history_id)
    if not history_entry:
        raise HTTPException(status_code=404, detail="History entry not found")

    track = await track_repo.get_by_id(history_entry.track_id)
    if not track:
        raise HTTPException(status_code=404, detail="Track not found")

    track.title = history_entry.title
    track.artist = history_entry.artist
    track.duration = history_entry.duration

    session.add(track)
    await session.commit()

    await notify_sse_from_worker(
        action_key="track_updated",
        message=f"Rolled back track '{track.title}'",
        level="info",
        payload=track.model_dump(),
    )

    return {"message": "Track rolled back successfully"}
