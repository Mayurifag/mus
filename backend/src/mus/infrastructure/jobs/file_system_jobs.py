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
    logger = logging.getLogger(__name__)
    logger.info(f"WORKER: Starting handle_file_created for: {file_path_str}")

    await _process_file_upsert(
        file_path_str, is_creation=True, skip_slow_metadata=skip_slow_metadata
    )

    logger.info(f"WORKER: Completed handle_file_created for: {file_path_str}")


@worker.task()
async def handle_file_modified(file_path_str: str):
    logger = logging.getLogger(__name__)
    logger.info(f"WORKER: Starting handle_file_modified for: {file_path_str}")

    if await check_app_write_lock(file_path_str):
        logger.info(
            f"WORKER: Skipping handle_file_modified (write lock active): {file_path_str}"
        )
        return

    await _process_file_upsert(
        file_path_str, is_creation=False, skip_slow_metadata=False
    )

    logger.info(f"WORKER: Completed handle_file_modified for: {file_path_str}")


@worker.task()
async def handle_file_deleted(file_path_str: str):
    logger = logging.getLogger(__name__)
    logger.info(f"WORKER: Starting handle_file_deleted for: {file_path_str}")

    track = await get_track_by_path(file_path_str)
    if track and track.id is not None:
        logger.info(f"WORKER: Found track {track.id} '{track.title}' for deletion")
        async with worker:
            await delete_track_by_id_internal.enqueue(
                track_id=track.id,
                track_title=track.title,
            )
        logger.info(
            f"WORKER: Enqueued delete_track_by_id_internal job for track {track.id}"
        )
    else:
        logger.warning(f"WORKER: No track found for file path: {file_path_str}")

    logger.info(f"WORKER: Completed handle_file_deleted for: {file_path_str}")


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
    logger.info(
        f"WORKER: Starting delete_track_by_id_internal for track {track_id} '{track_title}'"
    )

    await delete_track_from_db_by_id(track_id)
    logger.info(f"WORKER: Track {track_id} deleted from database")

    await notify_sse_from_worker(
        action_key="track_deleted",
        message=f"Deleted track '{track_title}'",
        level="info",
        payload={"id": track_id},
    )
    logger.info(f"WORKER: Completed delete_track_by_id_internal for track {track_id}")


@worker.task()
async def delete_track_with_files(track_id: int):
    logger = logging.getLogger(__name__)
    logger.info(f"WORKER: Starting delete_track_with_files for track {track_id}")

    track = await get_track_by_id(track_id)
    if not track:
        logger.warning(f"WORKER: Track {track_id} not found for deletion")
        return

    file_path = Path(track.file_path)
    if file_path.exists():
        file_path.unlink()
        logger.info(f"WORKER: Deleted file: {file_path}")

    await delete_track_from_db_by_id(track_id)
    logger.info(f"WORKER: Deleted track {track_id} from database")

    await notify_sse_from_worker(
        action_key="track_deleted",
        message=f"Deleted track '{track.title}'",
        level="success",
        payload={"id": track_id},
    )

    logger.info(f"WORKER: Completed delete_track_with_files for track {track_id}")


@worker.task()
async def update_track_path_by_id(track_id: int, new_path: str):
    logger = logging.getLogger(__name__)
    logger.info(
        f"WORKER: Starting update_track_path_by_id for track {track_id} to: {new_path}"
    )

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
        logger.info(f"WORKER: Completed update_track_path_by_id for track {track_id}")
    else:
        logger.warning(f"WORKER: Track {track_id} not found after path update")


async def _process_file_upsert(
    file_path_str: str, is_creation: bool = False, skip_slow_metadata: bool = False
):
    logger = logging.getLogger(__name__)
    logger.debug(
        f"WORKER: Processing file upsert for: {file_path_str} (creation: {is_creation})"
    )

    file_path = Path(file_path_str)
    metadata = extract_fast_metadata(file_path)
    if not metadata:
        logger.warning(f"WORKER: No metadata extracted for: {file_path_str}")
        return

    if is_creation:
        existing_track = await get_track_by_inode(metadata["inode"])
        if existing_track:
            logger.info(
                f"WORKER: Found existing track {existing_track.id} for inode, updating path"
            )
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
        logger.info(
            f"WORKER: Upserted track {upserted_track.id} '{upserted_track.title}'"
        )

        if not skip_slow_metadata:
            async with worker:
                await process_slow_metadata.enqueue(
                    track_id=upserted_track.id,
                )
            logger.debug(
                f"WORKER: Enqueued slow metadata processing for track {upserted_track.id}"
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
        logger.debug(f"WORKER: Sent SSE notification for track {upserted_track.id}")
    else:
        logger.error(f"WORKER: Failed to upsert track for: {file_path_str}")
