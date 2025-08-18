from fastapi import APIRouter, HTTPException, Depends
from pydantic import BaseModel

from src.mus.core.redis import get_redis_client
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.jobs.download_jobs import download_track_from_url
from src.mus.application.services.permissions_service import PermissionsService
from src.mus.infrastructure.api.dependencies import get_permissions_service

router = APIRouter(prefix="/api/v1/downloads", tags=["downloads"])


class DownloadUrlRequest(BaseModel):
    url: str


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
            # Release lock if job enqueue fails
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
