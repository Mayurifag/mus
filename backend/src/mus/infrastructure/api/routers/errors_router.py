from typing import List

from fastapi import APIRouter, HTTPException
from sqlmodel import select

from src.mus.application.dtos.track import TrackDTO
from src.mus.core.streaq_broker import worker
from src.mus.domain.entities.track import ProcessingStatus, Track
from src.mus.infrastructure.database import async_session_factory
from src.mus.infrastructure.jobs.metadata_jobs import process_slow_metadata
from src.mus.util.db_utils import get_track_by_id, update_track
from src.mus.util.track_dto_utils import create_track_dto_with_covers

router = APIRouter(prefix="/api/v1/errors", tags=["errors"])


@router.get("/tracks", response_model=List[TrackDTO])
async def get_errored_tracks():
    """Get all tracks that have processing errors."""
    async with async_session_factory() as session:
        result = await session.exec(
            select(Track).where(Track.processing_status == ProcessingStatus.ERROR)
        )
        tracks = result.all()

        return [create_track_dto_with_covers(track) for track in tracks]


@router.post("/tracks/{track_id}/requeue")
async def requeue_track(track_id: int):
    """Re-queue a failed track for processing."""
    track = await get_track_by_id(track_id)
    if not track:
        raise HTTPException(status_code=404, detail="Track not found")

    if not track.last_error:
        raise HTTPException(status_code=400, detail="Track has no error to retry")

    # Get the job name from the error
    job_name = track.last_error.get("job_name")
    if not job_name:
        raise HTTPException(status_code=400, detail="No job name found in error")

    # Clear the error and reset status
    track.last_error = None
    track.processing_status = ProcessingStatus.PENDING
    await update_track(track)

    # Re-enqueue the job
    if job_name == "process_slow_metadata":
        async with worker:
            await process_slow_metadata.enqueue(track_id=track_id)
    else:
        # For other job types, we might need to handle them differently
        # For now, default to slow metadata processing
        async with worker:
            await process_slow_metadata.enqueue(track_id=track_id)

    return {"message": f"Track '{track.title}' re-queued for processing"}
