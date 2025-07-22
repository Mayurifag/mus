from pathlib import Path

from arq import Ctx

from src.mus.core.redis import check_app_write_lock
from src.mus.domain.entities.track import ProcessingStatus, Track
from src.mus.util.metadata_extractor import extract_fast_metadata
from src.mus.util.db_utils import (
    get_track_by_path,
    get_track_by_inode,
    get_track_by_id,
    upsert_track,
    update_track,
    delete_track_from_db_by_id,
    update_track_path,
)
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker
from src.mus.util.track_dto_utils import create_track_dto_with_covers
from src.mus.core.arq_pool import get_arq_pool


async def handle_file_created(
    _ctx: Ctx, file_path_str: str, skip_slow_metadata: bool = False
):
    await _process_file_upsert(
        file_path_str, is_creation=True, skip_slow_metadata=skip_slow_metadata
    )


async def handle_file_modified(_ctx: Ctx, file_path_str: str):
    if await check_app_write_lock(file_path_str):
        return

    await _process_file_upsert(
        file_path_str, is_creation=False, skip_slow_metadata=False
    )


async def handle_file_deleted(_ctx: Ctx, file_path_str: str):
    track = await get_track_by_path(file_path_str)
    if track and track.id is not None:
        await delete_track_from_db_by_id(track.id)

        await notify_sse_from_worker(
            action_key="track_deleted",
            message=f"Deleted track '{track.title}'",
            level="info",
            payload={"track_id": track.id},
        )


async def handle_file_moved(_ctx: Ctx, old_path: str, new_path: str):
    track = await get_track_by_path(old_path)
    if track and track.id is not None:
        await update_track_path(track.id, new_path)

        updated_track = await get_track_by_id(track.id)
        if updated_track:
            track_dto = create_track_dto_with_covers(updated_track)
            await notify_sse_from_worker(
                action_key="track_updated",
                message=f"Moved track '{track.title}'",
                level="info",
                payload=track_dto.model_dump(),
            )


async def delete_track_with_files(_ctx: Ctx, track_id: int):
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
        payload={"track_id": track_id},
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
            arq_pool = await get_arq_pool()
            await arq_pool.enqueue_job(
                "process_slow_metadata",
                track_id=upserted_track.id,
                _queue_name="low_priority",
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
