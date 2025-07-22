import time

from src.mus.application.use_cases.process_track_metadata import (
    process_slow_metadata_for_track,
)
from src.mus.domain.entities.track import ProcessingStatus
from src.mus.util.db_utils import get_track_by_id, update_track
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker
from src.mus.util.track_dto_utils import create_track_dto_with_covers
from src.mus.core.streaq_broker import worker


@worker.task()
async def process_slow_metadata(track_id: int):
    try:
        track = await process_slow_metadata_for_track(track_id)

        if track:
            track.last_error = None
            track.processing_status = ProcessingStatus.COMPLETE
            await update_track(track)

            track_dto = create_track_dto_with_covers(track)
            await notify_sse_from_worker(
                action_key="track_updated",
                message=None,
                level="info",
                payload=track_dto.model_dump(),
            )

    except Exception as e:
        track = await get_track_by_id(track_id)
        if track:
            track.last_error = {
                "timestamp": int(time.time()),
                "job_name": "process_slow_metadata",
                "error_message": str(e),
            }
            track.processing_status = ProcessingStatus.ERROR
            await update_track(track)

            await notify_sse_from_worker(
                action_key="track_error",
                message=f"Failed to process metadata for '{track.title}': {str(e)}",
                level="error",
                payload={"track_id": track_id, "error": str(e)},
            )
