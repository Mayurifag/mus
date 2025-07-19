from enum import Enum
from fastapi import (
    APIRouter,
    Depends,
    HTTPException,
    Path,
    Request,
    status,
    UploadFile,
    File,
    Form,
)
from fastapi.responses import FileResponse, Response
from typing import List, Final, Dict, Any
import os
import hashlib
import io
import json

from mutagen._file import File as MutagenFile
from src.mus.application.dtos.track import TrackListDTO, TrackUpdateDTO
from src.mus.application.use_cases.edit_track_use_case import EditTrackUseCase
from src.mus.infrastructure.api.dependencies import get_track_repository
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.util.filename_utils import generate_track_filename
from src.mus.util.file_validation import validate_upload_file
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

V1_TO_EASY_MAPPING: Final = {
    "title": "title",
    "artist": "artist",
    "album": "album",
    "year": "date",
    "genre": "genre",
    "comment": "comment",
}

router = APIRouter(prefix="/api/v1/tracks", tags=["tracks"])


@router.get("", response_model=List[TrackListDTO])
async def get_tracks(
    track_repository: SQLiteTrackRepository = Depends(get_track_repository),
) -> List[TrackListDTO]:
    rows = await track_repository.get_all()

    track_dtos = []
    for row in rows:
        track_dto = TrackListDTO.model_validate(row)

        if row.has_cover:
            cover_base = f"/api/v1/tracks/{row.id}/covers"
            cache_param = f"?v={row.updated_at}"
            track_dto.cover_small_url = f"{cover_base}/small.webp{cache_param}"
            track_dto.cover_original_url = f"{cover_base}/original.webp{cache_param}"

        track_dtos.append(track_dto)

    return track_dtos


@router.get("/{track_id}/stream")
async def stream_track(
    request: Request,
    track_id: int = Path(..., gt=0),
    track_repository: SQLiteTrackRepository = Depends(get_track_repository),
):
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

    file_stat = await asyncio.to_thread(os.stat, track.file_path)
    etag = _generate_etag(track.file_path, file_stat.st_size, file_stat.st_mtime)

    if_none_match = request.headers.get("if-none-match")
    if if_none_match and if_none_match.strip('"') == etag:
        return Response(status_code=304)

    return FileResponse(
        path=track.file_path,
        media_type=content_type,
        filename=os.path.basename(track.file_path),
        headers={
            "ETag": f'"{etag}"',
            "Cache-Control": "public, max-age=86400, immutable",
            "Accept-Ranges": "bytes",
        },
    )


def _generate_etag(file_path: str, file_size: int, mtime: float) -> str:
    etag_data = f"{file_path}:{file_size}:{mtime}"
    return hashlib.md5(etag_data.encode(), usedforsecurity=False).hexdigest()


@router.get(
    "/{track_id}/covers/{size}.webp", name="get_track_cover", response_model=None
)
async def get_track_cover(
    request: Request,
    track_id: int = Path(..., gt=0),
    size: CoverSize = Path(...),
    track_repository: SQLiteTrackRepository = Depends(get_track_repository),
):
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

    file_stat = await asyncio.to_thread(os.stat, cover_path)
    file_size = file_stat.st_size
    file_mtime = file_stat.st_mtime

    etag = _generate_etag(str(cover_path), file_size, file_mtime)

    if_none_match = request.headers.get("if-none-match")
    if if_none_match and if_none_match.strip('"') == etag:
        return Response(status_code=304)

    return FileResponse(
        path=cover_path,
        media_type="image/webp",
        filename=f"cover_{size.value}.webp",
        headers={
            "ETag": f'"{etag}"',
            "Cache-Control": "public, max-age=31536000, immutable",
        },
    )


@router.patch("/{track_id}")
async def update_track(
    update_data: TrackUpdateDTO,
    track_id: int = Path(..., gt=0),
    track_repository: SQLiteTrackRepository = Depends(get_track_repository),
) -> Dict[str, Any]:
    use_case = EditTrackUseCase(track_repository)
    return await use_case.execute(track_id, update_data)


@router.post("/upload")
async def upload_track(
    title: str = Form(...),
    artist: str = Form(...),
    file: UploadFile = File(...),
    save_only_essentials: bool = Form(True),
    raw_tags: str = Form(None),
) -> Dict[str, Any]:
    # Validate file using shared utility
    extension = validate_upload_file(file)

    # Read file content into buffer
    file_content = await file.read()
    buffer = io.BytesIO(file_content)

    # Use mutagen to update metadata
    try:
        audio = MutagenFile(buffer, easy=True)
        if not audio:
            raise HTTPException(status_code=400, detail="Invalid audio file format")

        if save_only_essentials:
            # Clear all existing tags and set only essentials
            audio.clear()
            audio["title"] = title
            audio["artist"] = artist
        else:
            # Apply raw tags if provided
            if raw_tags:
                try:
                    tags_data = json.loads(raw_tags)

                    # Apply v2 tags if present
                    if "v2" in tags_data and isinstance(tags_data["v2"], dict):
                        for key, value in tags_data["v2"].items():
                            if key != "APIC":  # Skip picture data
                                if isinstance(value, list) and len(value) > 0:
                                    audio[key] = value[0]
                                elif not isinstance(value, list):
                                    audio[key] = value

                    # Apply v1 tags if present and no v2 equivalent
                    if "v1" in tags_data and isinstance(tags_data["v1"], dict):
                        for v1_key, easy_key in V1_TO_EASY_MAPPING.items():
                            if v1_key in tags_data["v1"] and easy_key not in audio:
                                audio[easy_key] = tags_data["v1"][v1_key]

                except (json.JSONDecodeError, KeyError, TypeError) as e:
                    raise HTTPException(
                        status_code=400, detail=f"Invalid raw tags format: {str(e)}"
                    )

            # Always ensure title and artist are set from form
            audio["title"] = title
            audio["artist"] = artist

        # Save changes back to buffer
        buffer.seek(0)
        audio.save(buffer)
        buffer.seek(0)
    except Exception as e:
        raise HTTPException(
            status_code=400, detail=f"Failed to process audio file: {str(e)}"
        )

    # Generate filename using the same format as edit track functionality
    try:
        filename = generate_track_filename(artist, title, extension)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

    file_path = settings.MUSIC_DIR_PATH / filename

    # Save file to music directory
    try:
        with open(file_path, "wb") as f:
            f.write(buffer.getvalue())
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to save file: {str(e)}")

    return {"success": True, "message": "File uploaded and queued for processing."}
