import asyncio
import hashlib
import io
import os
from enum import Enum
from typing import Any, Dict, Final, List

from fastapi import (
    APIRouter,
    Depends,
    File,
    Form,
    HTTPException,
    Path,
    Request,
    UploadFile,
    status,
)
from fastapi.responses import FileResponse, Response
from mutagen._file import File as MutagenFile

from src.mus.application.dtos.track import TrackListDTO, TrackUpdateDTO
from src.mus.application.use_cases.edit_track_use_case import EditTrackUseCase
from src.mus.application.services.permissions_service import PermissionsService
from src.mus.config import settings
from src.mus.core.redis import set_app_write_lock
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.api.dependencies import get_track_repository, get_permissions_service
from src.mus.infrastructure.jobs.file_system_jobs import (
    delete_track_with_files,
    handle_file_created,
)
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.util.file_validation import validate_upload_file
from src.mus.util.filename_utils import generate_track_filename


class CoverSize(str, Enum):
    SMALL = "small"
    ORIGINAL = "original"


AUDIO_CONTENT_TYPES: Final = {
    ".mp3": "audio/mpeg",
    ".flac": "audio/flac",
    ".wav": "audio/wav",
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
    permissions_service: PermissionsService = Depends(get_permissions_service),
) -> Dict[str, Any]:
    if not permissions_service.can_write_music_files:
        raise HTTPException(status_code=403, detail="Music directory is read-only")
    use_case = EditTrackUseCase(track_repository, permissions_service)
    return await use_case.execute(track_id, update_data)


@router.delete("/{track_id}", status_code=status.HTTP_202_ACCEPTED)
async def delete_track(
    track_id: int = Path(..., gt=0),
    permissions_service: PermissionsService = Depends(get_permissions_service),
) -> Response:
    if not permissions_service.can_write_music_files:
        raise HTTPException(status_code=403, detail="Music directory is read-only")
    async with worker:
        await delete_track_with_files.enqueue(track_id=track_id)
    return Response(status_code=status.HTTP_202_ACCEPTED)


@router.post("/upload")
async def upload_track(
    title: str = Form(...),
    artist: str = Form(...),
    file: UploadFile = File(...),
    permissions_service: PermissionsService = Depends(get_permissions_service),
) -> Dict[str, Any]:
    if not permissions_service.can_write_music_files:
        raise HTTPException(status_code=403, detail="Music directory is read-only")
    extension = validate_upload_file(file)

    file_content = await file.read()
    buffer = io.BytesIO(file_content)

    try:
        audio = MutagenFile(buffer, easy=True)
        if not audio:
            raise HTTPException(status_code=400, detail="Invalid audio file format")

        audio["title"] = title
        audio["artist"] = artist

        buffer.seek(0)
        audio.save(buffer)
        buffer.seek(0)
    except Exception as e:
        raise HTTPException(
            status_code=400, detail=f"Failed to process audio file: {str(e)}"
        )

    try:
        filename = generate_track_filename(artist, title, extension)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

    file_path = settings.MUSIC_DIR_PATH / filename

    await set_app_write_lock(str(file_path))

    try:
        with open(file_path, "wb") as f:
            f.write(buffer.getvalue())
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to save file: {str(e)}")

    async with worker:
        await handle_file_created.enqueue(
            file_path_str=str(file_path),
            skip_slow_metadata=True,
        )

    return {"success": True, "message": "File uploaded and queued for processing."}
