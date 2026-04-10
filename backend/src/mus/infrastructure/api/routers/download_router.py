import asyncio
import json
import logging

from fastapi import APIRouter, HTTPException, Depends
from pydantic import BaseModel

from src.mus.core.redis import get_redis_client
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.jobs.download_jobs import download_track_from_url
from src.mus.application.services.permissions_service import PermissionsService
from src.mus.infrastructure.api.dependencies import get_permissions_service
from src.mus.domain.services.title_cleaning_service import (
    clean_video_title,
    extract_artist_title,
)
from src.mus.domain.services.ollama_service import parse_track_metadata

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/v1/downloads", tags=["downloads"])


class DownloadUrlRequest(BaseModel):
    url: str


class MetadataRequest(BaseModel):
    url: str


class ConfirmDownloadRequest(BaseModel):
    url: str
    title: str
    artist: str


class MetadataResponse(BaseModel):
    title: str
    artist: str
    thumbnail_url: str | None
    duration: float | None


async def _fetch_raw_metadata(url: str) -> dict:
    proc = await asyncio.create_subprocess_exec(
        "yt-dlp",
        "--dump-json",
        "--no-playlist",
        url,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=30)
    except asyncio.TimeoutError:
        proc.kill()
        await proc.wait()
        raise
    if proc.returncode != 0:
        raise RuntimeError("yt-dlp exited with non-zero status")
    return json.loads(stdout)


@router.post("/metadata", response_model=MetadataResponse)
async def fetch_metadata(
    request: MetadataRequest,
    permissions_service: PermissionsService = Depends(get_permissions_service),
):
    if not permissions_service.can_write_music_files:
        raise HTTPException(
            status_code=403,
            detail="Download not available - insufficient write permissions to music directory",
        )
    try:
        data = await _fetch_raw_metadata(request.url)
    except asyncio.TimeoutError:
        raise HTTPException(status_code=504, detail="Metadata fetch timed out")
    except (RuntimeError, json.JSONDecodeError):
        raise HTTPException(status_code=422, detail="yt-dlp failed to fetch metadata")

    raw_title = data.get("title", "")
    raw_artist = data.get("artist")
    channel_name = data.get("channel") or data.get("uploader") or ""

    llm_result = await parse_track_metadata(raw_title, channel_name)
    if llm_result is not None:
        artist, title = llm_result
        logger.debug("LLM parsed metadata: artist=%r title=%r", artist, title)
    elif raw_artist:
        title = clean_video_title(raw_title)
        artist = raw_artist
    else:
        artist, title = extract_artist_title(raw_title, channel_name)

    return MetadataResponse(
        title=title,
        artist=artist,
        thumbnail_url=data.get("thumbnail"),
        duration=data.get("duration"),
    )


@router.post("/url", status_code=202)
async def initiate_download(
    request: DownloadUrlRequest,
    permissions_service: PermissionsService = Depends(get_permissions_service),
):
    if not permissions_service.can_write_music_files:
        raise HTTPException(
            status_code=403,
            detail="Download not available - insufficient write permissions to music directory",
        )

    client = await get_redis_client()
    try:
        lock_key = "download_lock:global"
        lock_acquired = await client.set(lock_key, "1", ex=600, nx=True)
        if not lock_acquired:
            raise HTTPException(status_code=429, detail="Download already in progress")
        try:
            async with worker:
                await download_track_from_url.enqueue(url=request.url)
        except Exception as e:
            await client.delete(lock_key)
            if "Missing function" in str(e) or "download_track_from_url" in str(e):
                raise HTTPException(
                    status_code=503,
                    detail="Download service is temporarily unavailable",
                )
            raise HTTPException(
                status_code=500, detail=f"Failed to start download: {str(e)}"
            )
        return {"status": "accepted"}
    finally:
        await client.aclose()


@router.post("/confirm", status_code=202)
async def confirm_download(
    request: ConfirmDownloadRequest,
    permissions_service: PermissionsService = Depends(get_permissions_service),
):
    if not permissions_service.can_write_music_files:
        raise HTTPException(
            status_code=403,
            detail="Download not available - insufficient write permissions to music directory",
        )

    client = await get_redis_client()
    try:
        lock_key = "download_lock:global"
        lock_acquired = await client.set(lock_key, "1", ex=600, nx=True)
        if not lock_acquired:
            raise HTTPException(status_code=429, detail="Download already in progress")
        try:
            async with worker:
                await download_track_from_url.enqueue(
                    url=request.url,
                    title=request.title,
                    artist=request.artist,
                )
        except Exception as e:
            await client.delete(lock_key)
            if "Missing function" in str(e) or "download_track_from_url" in str(e):
                raise HTTPException(
                    status_code=503,
                    detail="Download service is temporarily unavailable",
                )
            raise HTTPException(
                status_code=500, detail=f"Failed to start download: {str(e)}"
            )
        return {"status": "accepted"}
    finally:
        await client.aclose()
