from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from src.mus.core.redis import get_redis_client
from src.mus.core.streaq_broker import worker
from src.mus.infrastructure.jobs.download_jobs import download_track_from_url

router = APIRouter(prefix="/api/v1/downloads", tags=["downloads"])


class DownloadUrlRequest(BaseModel):
    url: str


@router.post("/url", status_code=202)
async def initiate_download(request: DownloadUrlRequest):
    client = await get_redis_client()
    try:
        lock_key = "download_lock:global"
        lock_acquired = await client.set(lock_key, "1", ex=600, nx=True)

        if not lock_acquired:
            raise HTTPException(status_code=429, detail="Download already in progress")

        async with worker:
            await download_track_from_url.enqueue(url=request.url)

        return {"status": "accepted"}
    finally:
        await client.aclose()
