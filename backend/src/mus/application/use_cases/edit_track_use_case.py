import os
import re
import time
from pathlib import Path
from typing import Dict, Any

import mutagen
from fastapi import HTTPException

from src.mus.application.dtos.track import TrackUpdateDTO
from src.mus.domain.entities.track_history import TrackHistory
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.util.db_utils import add_track_history


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

        try:
            audio = mutagen.File(track.file_path, easy=True)
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

        new_file_path = track.file_path
        if update_data.rename_file:

            def sanitize(name: str) -> str:
                return re.sub(r'[<>:"/\\|?*]', "", name)

            new_title = (
                update_data.title if update_data.title is not None else track.title
            )
            new_artist = (
                update_data.artist if update_data.artist is not None else track.artist
            )

            formatted_artists = ", ".join(new_artist.split(";"))
            new_name = f"{sanitize(formatted_artists)} - {sanitize(new_title)}{Path(track.file_path).suffix}"

            if len(new_name) > 255:
                raise HTTPException(status_code=400, detail="Filename too long")

            new_file_path = str(Path(track.file_path).parent / new_name)
            os.rename(track.file_path, new_file_path)

        # Update track object
        if "title" in changes_delta:
            track.title = update_data.title
        if "artist" in changes_delta:
            track.artist = update_data.artist
        if new_file_path != track.file_path:
            track.file_path = new_file_path

        await self.track_repo.session.commit()
        await self.track_repo.session.refresh(track)

        # Create history entry
        history_entry = TrackHistory(
            track_id=track.id,
            event_type="edit",
            changes={
                "title": {"old": track.title, "new": update_data.title}
                if "title" in changes_delta
                else None,
                "artist": {"old": track.artist, "new": update_data.artist}
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

        return {"status": "success", "track": track}
