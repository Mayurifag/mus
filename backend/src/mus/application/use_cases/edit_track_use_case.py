import os
import time
from pathlib import Path
from typing import Dict, Any

from mutagen._file import File as MutagenFile
from fastapi import HTTPException

from src.mus.application.dtos.track import TrackUpdateDTO
from src.mus.application.services.permissions_service import PermissionsService
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.api.sse_handler import broadcast_sse_event
from src.mus.util.track_dto_utils import create_track_dto_with_covers
from src.mus.util.filename_utils import generate_track_filename
from src.mus.core.redis import set_app_write_lock


class EditTrackUseCase:
    def __init__(self, track_repo: SQLiteTrackRepository, permissions_service: PermissionsService):
        self.track_repo = track_repo
        self.permissions_service = permissions_service

    async def execute(
        self, track_id: int, update_data: TrackUpdateDTO
    ) -> Dict[str, Any]:
        track = await self.track_repo.get_by_id(track_id)
        if not track:
            raise HTTPException(status_code=404, detail="Track not found")

        changes_delta = update_data.model_dump(
            exclude_unset=True, exclude={"rename_file"}
        )

        if "title" in changes_delta and changes_delta["title"] == track.title:
            del changes_delta["title"]
        if "artist" in changes_delta and changes_delta["artist"] == track.artist:
            del changes_delta["artist"]

        if not changes_delta and not update_data.rename_file:
            return {"status": "no_changes"}

        try:
            audio = MutagenFile(track.file_path, easy=True)
            if not audio:
                raise HTTPException(status_code=400, detail="Unable to read audio file")
        except FileNotFoundError:
            raise HTTPException(status_code=404, detail="Audio file not found")

        if "title" in changes_delta:
            audio["title"] = update_data.title
        if "artist" in changes_delta:
            audio["artist"] = update_data.artist

        if changes_delta:
            await set_app_write_lock(track.file_path)
            audio.save()

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
            await set_app_write_lock(track.file_path)
            await set_app_write_lock(new_file_path)
            os.rename(track.file_path, new_file_path)

        if "title" in changes_delta and update_data.title is not None:
            track.title = update_data.title
        if "artist" in changes_delta and update_data.artist is not None:
            track.artist = update_data.artist
        if new_file_path != track.file_path:
            track.file_path = new_file_path

        if changes_delta or new_file_path != track.file_path:
            track.updated_at = int(time.time())

        await self.track_repo.session.commit()
        await self.track_repo.session.refresh(track)
        track_dto = create_track_dto_with_covers(track)

        await broadcast_sse_event(
            action_key="track_updated",
            message_to_show=f"Updated track '{track.title}'",
            message_level="info",
            action_payload=track_dto.model_dump(),
        )

        return {"status": "success", "track": track}
