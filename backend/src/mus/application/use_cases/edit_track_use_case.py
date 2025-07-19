import os
import time
import asyncio
from pathlib import Path
from typing import Dict, Any

from mutagen._file import File as MutagenFile
from fastapi import HTTPException

from src.mus.application.dtos.track import TrackUpdateDTO
from src.mus.domain.entities.track_history import TrackHistory
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.util.db_utils import add_track_history
from src.mus.infrastructure.api.sse_handler import broadcast_sse_event
from src.mus.util.track_dto_utils import create_track_dto_with_covers
from src.mus.util.filename_utils import generate_track_filename


class EditTrackUseCase:
    def __init__(self, track_repo: SQLiteTrackRepository):
        self.track_repo = track_repo

    async def execute(
        self, track_id: int, update_data: TrackUpdateDTO
    ) -> Dict[str, Any]:
        track = await self.track_repo.get_by_id(track_id)
        if not track:
            raise HTTPException(status_code=404, detail="Track not found")

        if not os.access(track.file_path, os.W_OK):
            raise HTTPException(status_code=403, detail="File is not writable")

        changes_delta = update_data.model_dump(
            exclude_unset=True, exclude={"rename_file"}
        )

        # Remove fields that haven't actually changed
        if "title" in changes_delta and changes_delta["title"] == track.title:
            del changes_delta["title"]
        if "artist" in changes_delta and changes_delta["artist"] == track.artist:
            del changes_delta["artist"]

        if not changes_delta and not update_data.rename_file:
            return {"status": "no_changes"}

        # Store original file timestamps before modifying
        original_stat = await asyncio.to_thread(os.stat, track.file_path)
        original_atime = original_stat.st_atime
        original_mtime = original_stat.st_mtime

        try:
            audio = MutagenFile(track.file_path, easy=True)
            if not audio:
                raise HTTPException(status_code=400, detail="Unable to read audio file")
        except FileNotFoundError:
            raise HTTPException(status_code=404, detail="Audio file not found")

        # Update tags if changes exist
        if "title" in changes_delta:
            audio["title"] = update_data.title
        if "artist" in changes_delta:
            audio["artist"] = update_data.artist

        if changes_delta:
            audio.save()
            # Restore original file timestamps to prevent watchdog from detecting this as an external change
            await asyncio.to_thread(
                os.utime, track.file_path, (original_atime, original_mtime)
            )

        new_file_path = track.file_path
        if update_data.rename_file:
            new_title = (
                update_data.title if update_data.title is not None else track.title
            )
            new_artist = (
                update_data.artist if update_data.artist is not None else track.artist
            )

            try:
                new_name = generate_track_filename(
                    new_artist, new_title, Path(track.file_path).suffix
                )
            except ValueError as e:
                raise HTTPException(status_code=400, detail=str(e))

            new_file_path = str(Path(track.file_path).parent / new_name)
            os.rename(track.file_path, new_file_path)

        # Store original values before updating
        original_title = track.title
        original_artist = track.artist

        # Update track object
        if "title" in changes_delta and update_data.title is not None:
            track.title = update_data.title
        if "artist" in changes_delta and update_data.artist is not None:
            track.artist = update_data.artist
        if new_file_path != track.file_path:
            track.file_path = new_file_path

        # Set updated timestamp if any changes were made
        if changes_delta or new_file_path != track.file_path:
            track.updated_at = int(time.time())

        await self.track_repo.session.commit()
        await self.track_repo.session.refresh(track)

        # Ensure track has an ID after refresh
        if track.id is None:
            raise HTTPException(
                status_code=500, detail="Track ID is missing after save"
            )

        # Create history entry
        history_entry = TrackHistory(
            track_id=track.id,
            event_type="edit",
            changes={
                "title": {"old": original_title, "new": update_data.title}
                if "title" in changes_delta
                else None,
                "artist": {"old": original_artist, "new": update_data.artist}
                if "artist" in changes_delta
                else None,
                "file_renamed": update_data.rename_file,
            },
            filename=Path(new_file_path).name,
            title=track.title,
            artist=track.artist,
            duration=track.duration,
            changed_at=int(time.time()),
            full_snapshot={
                "title": track.title,
                "artist": track.artist,
                "duration": track.duration,
                "file_path": track.file_path,
                "has_cover": track.has_cover,
            },
        )
        await add_track_history(history_entry)
        # Create TrackListDTO with proper cover URLs for SSE event
        track_dto = create_track_dto_with_covers(track)

        await broadcast_sse_event(
            action_key="track_updated",
            message_to_show=f"Updated track '{track.title}'",
            message_level="info",
            action_payload=track_dto.model_dump(),
        )

        return {"status": "success", "track": track}
