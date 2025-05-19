from enum import Enum
from fastapi import APIRouter, Depends, HTTPException, Path, Request, status
from fastapi.responses import FileResponse
from typing import List, Final
import os
from src.mus.application.dtos.track import TrackDTO
from src.mus.infrastructure.api.dependencies import get_track_repository
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.config import settings
import asyncio


class CoverSize(str, Enum):
    SMALL = "small"
    ORIGINAL = "original"


AUDIO_CONTENT_TYPES: Final = {
    ".mp3": "audio/mpeg",
    ".flac": "audio/flac",
    ".wav": "audio/wav",
}


router = APIRouter(prefix="/api/v1/tracks", tags=["tracks"])


@router.get("", response_model=List[TrackDTO])
async def get_tracks(
    request: Request,
    track_repository: SQLiteTrackRepository = Depends(get_track_repository),
) -> List[TrackDTO]:
    tracks = await track_repository.get_all()

    track_dtos = []
    for track in tracks:
        track_dto = TrackDTO.model_validate(track)

        if track.has_cover:
            track_dto.cover_small_url = (
                f"{request.url_for('get_track_cover', track_id=track.id, size='small')}"
            )
            track_dto.cover_original_url = f"{request.url_for('get_track_cover', track_id=track.id, size='original')}"

        track_dtos.append(track_dto)

    return track_dtos


@router.get("/{track_id}/stream")
async def stream_track(
    track_id: int = Path(..., gt=0),
    track_repository: SQLiteTrackRepository = Depends(get_track_repository),
) -> FileResponse:
    track = await track_repository.get_by_id(track_id)
    if not track:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Track with ID {track_id} not found",
        )

    if not os.path.isfile(track.file_path):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Audio file for track {track_id} not found",
        )

    file_ext = os.path.splitext(track.file_path)[1].lower()
    content_type = AUDIO_CONTENT_TYPES.get(file_ext, "audio/mpeg")

    return FileResponse(
        path=track.file_path,
        media_type=content_type,
        filename=os.path.basename(track.file_path),
    )


@router.get("/{track_id}/covers/{size}.webp", name="get_track_cover")
async def get_track_cover(
    track_id: int = Path(..., gt=0),
    size: CoverSize = Path(...),
    track_repository: SQLiteTrackRepository = Depends(get_track_repository),
) -> FileResponse:
    track = await track_repository.get_by_id(track_id)
    if not track:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Track with ID {track_id} not found",
        )

    if not track.has_cover:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="This track has no cover image",
        )

    cover_file_name = f"{track_id}_{size.value}.webp"
    cover_path = settings.COVERS_DIR_PATH / cover_file_name

    if not await asyncio.to_thread(os.path.isfile, cover_path):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Cover image for track {track_id} with size {size} not found",
        )

    return FileResponse(
        path=cover_path, media_type="image/webp", filename=f"cover_{size.value}.webp"
    )
