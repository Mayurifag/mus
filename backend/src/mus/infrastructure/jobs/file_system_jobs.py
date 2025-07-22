import logging
from pathlib import Path

from src.mus.core.redis import check_app_write_lock
from src.mus.core.streaq_broker import worker
from src.mus.domain.entities.track import ProcessingStatus, Track
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker
from src.mus.infrastructure.jobs.metadata_jobs import process_slow_metadata
from src.mus.util.db_utils import (
    delete_track_from_db_by_id,
    get_track_by_id,
    get_track_by_inode,
    get_track_by_path,
    update_track,
    update_track_path,
    upsert_track,
)
from src.mus.util.metadata_extractor import extract_fast_metadata
from src.mus.util.track_dto_utils import create_track_dto_with_covers


@worker.task()
async def handle_file_created(file_path_str: str, skip_slow_metadata: bool = False):
    await _process_file_upsert(
        file_path_str, is_creation=True, skip_slow_metadata=skip_slow_metadata
    )


@worker.task()
async def handle_file_modified(file_path_str: str):
    if await check_app_write_lock(file_path_str):
        return

    await _process_file_upsert(
        file_path_str, is_creation=False, skip_slow_metadata=False
    )


@worker.task()
async def handle_file_deleted(file_path_str: str):
    logger = logging.getLogger(__name__)
    logger.info(f"ARQ: Processing file deletion for: {file_path_str}")

    track = await get_track_by_path(file_path_str)
    if track and track.id is not None:
        logger.info(f"Found track {track.id} '{track.title}' for deletion")
        async with worker:
            await delete_track_by_id_internal.enqueue(
                track_id=track.id,
                track_title=track.title,
            )
        logger.info(f"Enqueued delete_track_by_id_internal job for track {track.id}")
    else:
        logger.warning(f"No track found for file path: {file_path_str}")


@worker.task()
async def handle_file_moved(old_path: str, new_path: str):
    track = await get_track_by_path(old_path)
    if track and track.id is not None:
        async with worker:
            await update_track_path_by_id.enqueue(
                track_id=track.id,
                new_path=new_path,
            )


@worker.task()
async def delete_track_by_id_internal(track_id: int, track_title: str):
    logger = logging.getLogger(__name__)
    logger.info(f"ARQ: Deleting track {track_id} '{track_title}' from database")

    await delete_track_from_db_by_id(track_id)
    logger.info(f"ARQ: Track {track_id} deleted from database")

    await notify_sse_from_worker(
        action_key="track_deleted",
        message=f"Deleted track '{track_title}'",
        level="info",
        payload={"id": track_id},
    )
    logger.info(f"SSE notification sent for track {track_id} deletion")


@worker.task()
async def delete_track_with_files(track_id: int):
    track = await get_track_by_id(track_id)
    if not track:
        return

    file_path = Path(track.file_path)
    if file_path.exists():
        file_path.unlink()

    await delete_track_from_db_by_id(track_id)

    await notify_sse_from_worker(
        action_key="track_deleted",
        message=f"Deleted track '{track.title}'",
        level="success",
        payload={"id": track_id},
    )


@worker.task()
async def update_track_path_by_id(track_id: int, new_path: str):
    await update_track_path(track_id, new_path)

    updated_track = await get_track_by_id(track_id)
    if updated_track:
        track_dto = create_track_dto_with_covers(updated_track)
        await notify_sse_from_worker(
            action_key="track_updated",
            message=f"Moved track '{updated_track.title}'",
            level="info",
            payload=track_dto.model_dump(),
        )


async def _process_file_upsert(
    file_path_str: str, is_creation: bool = False, skip_slow_metadata: bool = False
):
    file_path = Path(file_path_str)
    metadata = extract_fast_metadata(file_path)
    if not metadata:
        return

    if is_creation:
        existing_track = await get_track_by_inode(metadata["inode"])
        if existing_track:
            existing_track.file_path = str(file_path)
            await update_track(existing_track)
            return

    existing_track = await get_track_by_path(str(file_path))

    track = Track(
        title=metadata["title"],
        artist=metadata["artist"],
        duration=metadata["duration"],
        file_path=str(file_path),
        added_at=metadata["added_at"],
        updated_at=metadata["added_at"],
        inode=metadata["inode"],
        processing_status=ProcessingStatus.PENDING,
    )
    upserted_track = await upsert_track(track)

    if upserted_track and upserted_track.id:
        if not skip_slow_metadata:
            async with worker:
                await process_slow_metadata.enqueue(
                    track_id=upserted_track.id,
                )

        action_key = "track_added" if is_creation else "track_updated"
        action_message = "Added" if is_creation else "Updated"
        level = "success" if is_creation else "info"
        track_dto = create_track_dto_with_covers(upserted_track)

        await notify_sse_from_worker(
            action_key=action_key,
            message=f"{action_message} track '{track.title}'",
            level=level,
            payload=track_dto.model_dump(),
        )
