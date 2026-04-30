import asyncio

from fastapi import APIRouter, Depends, HTTPException

from src.mus.application.services.permissions_service import PermissionsService
from src.mus.config import settings
from src.mus.infrastructure.api.dependencies import get_permissions_service

router = APIRouter(prefix="/api/v1/system", tags=["system"])


@router.get("/permissions")
async def get_permissions(
    service: PermissionsService = Depends(get_permissions_service),
) -> dict:
    return {"can_write_music_files": service.can_write_music_files}


async def _yt_dlp_version() -> str | None:
    try:
        proc = await asyncio.create_subprocess_exec(
            "yt-dlp",
            "--version",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=10)
    except (asyncio.TimeoutError, FileNotFoundError):
        return None
    if proc.returncode != 0:
        return None
    return stdout.decode().strip() or None


@router.get("/info")
async def get_system_info() -> dict:
    return {
        "commit_sha": settings.COMMIT_SHA,
        "yt_dlp_version": await _yt_dlp_version(),
    }


@router.post("/yt-dlp/update")
async def update_yt_dlp() -> dict:
    try:
        proc = await asyncio.create_subprocess_exec(
            "yt-dlp",
            "--update-to",
            "nightly",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=120)
    except asyncio.TimeoutError:
        raise HTTPException(status_code=504, detail="yt-dlp update timed out")
    except FileNotFoundError:
        raise HTTPException(status_code=500, detail="yt-dlp binary not found")
    if proc.returncode != 0:
        detail = stderr.decode().strip()[:300] or "yt-dlp update failed"
        raise HTTPException(status_code=500, detail=detail)
    return {
        "yt_dlp_version": await _yt_dlp_version(),
        "output": stdout.decode().strip(),
    }
