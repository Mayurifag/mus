import asyncio
import logging
import time
from contextlib import asynccontextmanager
from typing import Any, Dict

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from src.mus.application.use_cases.fast_initial_scan import FastInitialScanUseCase
from src.mus.application.use_cases.slow_initial_scan import SlowInitialScanUseCase
from src.mus.config import settings
from src.mus.infrastructure.api.permissions_singleton import permissions_service
from src.mus.infrastructure.api.routers import (
    auth_router,
    download_router,
    errors_router,
    permissions_router,
    player_router,
    track_router,
)
from src.mus.infrastructure.api.sse_handler import router as sse_router
from src.mus.infrastructure.database import create_db_and_tables
from src.mus.infrastructure.file_watcher.watcher import watch_music_directory
from src.mus.infrastructure.services.ytdlp_service import ytdlp_service

logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(_: FastAPI):
    await create_db_and_tables()

    await asyncio.to_thread(permissions_service.check_permissions)

    # Initialize yt-dlp-proxy on startup
    logger.info("Checking yt-dlp-proxy availability...")
    if not await ytdlp_service.ensure_ytdlp_proxy_available():
        logger.info("yt-dlp-proxy not available, running update script...")
        await ytdlp_service.run_update_script(max_workers=4)

    fast_scanner = await FastInitialScanUseCase.create_default()
    await fast_scanner.execute()

    tasks = [
        asyncio.create_task(SlowInitialScanUseCase(permissions_service).execute()),
        asyncio.create_task(watch_music_directory()),
    ]

    try:
        yield
    finally:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)


app = FastAPI(
    title="mus backend",
    description="FastAPI python backend for mus project",
    lifespan=lifespan,
)

if settings.APP_ENV != "production":
    app.add_middleware(
        CORSMiddleware,
        allow_origins=[settings.FRONTEND_ORIGIN],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

app.include_router(auth_router.router)
app.include_router(download_router.router)
app.include_router(player_router.router)
app.include_router(track_router.router)
app.include_router(permissions_router.router)
app.include_router(errors_router.router)
app.include_router(sse_router)


@app.get("/api/healthcheck.json", response_model=Dict[str, Any])
async def healthcheck() -> Dict[str, Any]:
    return {"status": "healthy", "timestamp": int(time.time())}
