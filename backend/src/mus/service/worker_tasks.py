import asyncio
from pathlib import Path
import time

from src.mus.config import settings
from src.mus.domain.entities.track import ProcessingStatus, Track
from src.mus.domain.entities.track_history import TrackHistory
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor

from src.mus.util.ffprobe_analyzer import get_accurate_duration
from src.mus.util.metadata_extractor import extract_fast_metadata
from src.mus.infrastructure.api.sse_handler import notify_sse_from_worker
from src.mus.util.db_utils import (
    get_track_by_id,
    get_track_by_path,
    get_track_by_inode,
    upsert_track,
    update_track,
    delete_track_by_path,
    update_track_path,
    add_track_history,
    prune_track_history,
)
from src.mus.util.queue_utils import enqueue_slow_metadata


async def process_slow_metadata(track_id: int):
    track = await get_track_by_id(track_id)
    if not track or not Path(track.file_path).exists():
        return

    file_path = Path(track.file_path)
    cover_processor = CoverProcessor(settings.COVERS_DIR_PATH)

    duration_task = asyncio.to_thread(get_accurate_duration, file_path)
    cover_task = cover_processor.extract_cover_from_file(file_path)

    accurate_duration, cover_data = await asyncio.gather(duration_task, cover_task)

    if accurate_duration > 0 and accurate_duration != track.duration:
        track.duration = accurate_duration

    track.has_cover = bool(
        cover_data
        and await cover_processor.process_and_save_cover(
            track_id, cover_data, file_path
        )
    )

    track.processing_status = ProcessingStatus.COMPLETE
    await update_track(track)
    await notify_sse_from_worker(
        action_key="track_updated",
        message=f"Processed metadata for '{track.title}'",
        level="success",
        payload=track.model_dump(),
    )


async def process_file_deletion(file_path_str: str):
    track = await get_track_by_path(file_path_str)
    if not track:
        return

    track_title = track.title
    track_id = await delete_track_by_path(file_path_str)
    if not track_id:
        return

    for suffix in ["_original.webp", "_small.webp"]:
        cover_path = settings.COVERS_DIR_PATH / f"{track_id}{suffix}"
        if await asyncio.to_thread(cover_path.exists):
            await asyncio.to_thread(cover_path.unlink)

    await notify_sse_from_worker(
        action_key="track_deleted",
        message=f"Deleted track '{track_title}'",
        level="info",
        payload={"id": track_id},
    )


async def process_file_move(src_path: str, dest_path: str):
    track = await get_track_by_path(src_path)
    if track:
        track_title = track.title
        updated_track = await update_track_path(src_path, dest_path)
        if updated_track is not None:
            await notify_sse_from_worker(
                action_key="track_updated",
                message=f"Moved track '{track_title}'",
                level="info",
                payload=updated_track.model_dump(),
            )


async def process_file_upsert(file_path_str: str, is_creation: bool = False):
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
        inode=metadata["inode"],
        processing_status=ProcessingStatus.METADATA_DONE,
    )
    upserted_track = await upsert_track(track)

    if not existing_track and upserted_track and upserted_track.id:
        history_entry = TrackHistory(
            track_id=upserted_track.id,
            event_type="initial_scan",
            changes=None,
            filename=Path(upserted_track.file_path).name,
            title=upserted_track.title,
            artist=upserted_track.artist,
            duration=upserted_track.duration,
            changed_at=int(time.time()),
            full_snapshot={
                "title": upserted_track.title,
                "artist": upserted_track.artist,
                "duration": upserted_track.duration,
                "file_path": upserted_track.file_path,
                "has_cover": upserted_track.has_cover,
            },
        )
        await add_track_history(history_entry)

    if existing_track and upserted_track and upserted_track.id:
        changes_dict = {}
        if existing_track.title != upserted_track.title:
            changes_dict["title"] = {
                "old": existing_track.title,
                "new": upserted_track.title,
            }
        if existing_track.artist != upserted_track.artist:
            changes_dict["artist"] = {
                "old": existing_track.artist,
                "new": upserted_track.artist,
            }
        if existing_track.duration != upserted_track.duration:
            changes_dict["duration"] = {
                "old": existing_track.duration,
                "new": upserted_track.duration,
            }

        if changes_dict:
            history_entry = TrackHistory(
                track_id=upserted_track.id,
                title=existing_track.title,
                artist=existing_track.artist,
                duration=existing_track.duration,
                changed_at=int(time.time()),
                event_type="metadata_update",
                filename=Path(upserted_track.file_path).name,
                changes=changes_dict,
                full_snapshot={
                    "title": upserted_track.title,
                    "artist": upserted_track.artist,
                    "duration": upserted_track.duration,
                    "file_path": upserted_track.file_path,
                    "has_cover": upserted_track.has_cover,
                },
            )
            await add_track_history(history_entry)
            await prune_track_history(upserted_track.id, 5)

    if upserted_track and upserted_track.id:
        enqueue_slow_metadata(upserted_track.id)
        action_key = "track_added" if is_creation else "track_updated"
        action_message = "Added" if is_creation else "Updated"
        level = "success" if is_creation else "info"
        await notify_sse_from_worker(
            action_key=action_key,
            message=f"{action_message} track '{track.title}'",
            level=level,
            payload=upserted_track.model_dump(),
        )


async def process_file_created(file_path_str: str):
    await process_file_upsert(file_path_str, is_creation=True)


async def process_file_modified(file_path_str: str):
    await process_file_upsert(file_path_str, is_creation=False)
