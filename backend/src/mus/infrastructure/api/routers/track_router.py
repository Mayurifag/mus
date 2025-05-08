from enum import Enum
from fastapi import APIRouter, Depends, HTTPException, Path, Request, status
from fastapi.responses import FileResponse
from typing import List
import os

from src.mus.application.dtos.track import TrackDTO
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


class CoverSize(str, Enum):
    """Valid cover image sizes."""

    SMALL = "small"
    ORIGINAL = "original"


router = APIRouter(prefix="/api/v1/tracks", tags=["tracks"])


@router.get("", response_model=List[TrackDTO])
async def get_tracks(
    request: Request,
    track_repository: SQLiteTrackRepository = Depends(),
) -> List[TrackDTO]:
    """Get all tracks."""
    tracks = await track_repository.get_all()

    # Convert entities to DTOs and add cover URLs
    track_dtos = []
    for track in tracks:
        track_dto = TrackDTO.model_validate(track)

        # If track has cover, construct URLs
        if track.has_cover:
            track_dto.cover_small_url = (
                f"{request.url_for('get_track_cover', track_id=track.id, size='small')}"
            )
            track_dto.cover_original_url = f"{request.url_for('get_track_cover', track_id=track.id, size='original')}"

        track_dtos.append(track_dto)

    return track_dtos


@router.get("/{track_id}/covers/{size}.webp", name="get_track_cover")
async def get_track_cover(
    track_id: int = Path(..., gt=0),
    size: CoverSize = Path(...),
    track_repository: SQLiteTrackRepository = Depends(),
) -> FileResponse:
    """
    Get a track cover image in WebP format.

    Size can be 'small' (80x80) or 'original' (original dimensions).
    """
    # Get track from database
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

    # Construct path to cover file
    # CoverProcessor saves covers to ./data/covers/{track_id}_{size}.webp
    cover_path = f"./data/covers/{track_id}_{size.value}.webp"

    # Check if file exists
    if not os.path.isfile(cover_path):
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Cover image for track {track_id} with size {size} not found",
        )

    return FileResponse(
        cover_path, media_type="image/webp", filename=f"cover_{size.value}.webp"
    )
