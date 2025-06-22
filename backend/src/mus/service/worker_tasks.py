import asyncio
from pathlib import Path

from src.mus.config import settings
from src.mus.domain.entities.track import ProcessingStatus, Track
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
)
from src.mus.util.queue_utils import enqueue_slow_metadata


async def process_slow_metadata(track_id: int):
    track = await get_track_by_id(track_id)
    if not track or not Path(track.file_path).exists():
        return

    file_path = Path(track.file_path)

    accurate_duration = await asyncio.to_thread(get_accurate_duration, file_path)
    if accurate_duration > 0 and accurate_duration != track.duration:
        track.duration = accurate_duration

    cover_processor = CoverProcessor(settings.COVERS_DIR_PATH)
    cover_data = await cover_processor.extract_cover_from_file(file_path)
    track.has_cover = bool(
        cover_data
        and await cover_processor.process_and_save_cover(
            track_id, cover_data, file_path
        )
    )

    track.processing_status = ProcessingStatus.COMPLETE
    await update_track(track)
    await notify_sse_from_worker(
        "reload_tracks", f"Processed metadata for '{track.title}'", "success"
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
        "reload_tracks", f"Deleted track '{track_title}'", "info"
    )


async def process_file_move(src_path: str, dest_path: str):
    track = await get_track_by_path(src_path)
    if track:
        track_title = track.title
        success = await update_track_path(src_path, dest_path)
        if success:
            await notify_sse_from_worker(
                "reload_tracks", f"Moved track '{track_title}'", "info"
            )


async def process_file_created(file_path_str: str):
    file_path = Path(file_path_str)
    metadata = extract_fast_metadata(file_path)
    if not metadata:
        return

    existing_track = await get_track_by_inode(metadata["inode"])
    if existing_track:
        existing_track.file_path = str(file_path)
        await update_track(existing_track)
        return

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

    if upserted_track and upserted_track.id:
        enqueue_slow_metadata(upserted_track.id)
        await notify_sse_from_worker(
            "reload_tracks", f"Added track '{track.title}'", "success"
        )


async def process_file_modified(file_path_str: str):
    file_path = Path(file_path_str)
    metadata = extract_fast_metadata(file_path)
    if not metadata:
        return

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

    if upserted_track and upserted_track.id:
        enqueue_slow_metadata(upserted_track.id)
        await notify_sse_from_worker(
            "reload_tracks", f"Updated track '{track.title}'", "info"
        )
