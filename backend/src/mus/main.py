import asyncio
import logging
from contextlib import asynccontextmanager
from typing import Any, Dict

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from src.mus.application.services.permissions_service import PermissionsService
from src.mus.application.use_cases.fast_initial_scan import FastInitialScanUseCase
from src.mus.application.use_cases.slow_initial_scan import SlowInitialScanUseCase
from src.mus.config import settings
from src.mus.infrastructure.api.routers import (
    auth_router,
    errors_router,
    permissions_router,
    player_router,
    track_router,
)
from src.mus.infrastructure.api.sse_handler import router as sse_router
from src.mus.infrastructure.file_watcher.watcher import watch_music_directory

logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(_: FastAPI):
    permissions_service = PermissionsService()
    await asyncio.to_thread(permissions_service.check_write_permissions)

    fast_scanner = await FastInitialScanUseCase.create_default()
    await fast_scanner.execute()

    slow_scanner = SlowInitialScanUseCase()
    slow_scan_task = asyncio.create_task(slow_scanner.execute())

    watcher_task = asyncio.create_task(watch_music_directory())

    try:
        yield
    finally:
        watcher_task.cancel()
        slow_scan_task.cancel()
        try:
            await watcher_task
        except asyncio.CancelledError:
            pass
        try:
            await slow_scan_task
        except asyncio.CancelledError:
            pass


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
app.include_router(player_router.router)
app.include_router(track_router.router)
app.include_router(permissions_router.router)
app.include_router(errors_router.router)
app.include_router(sse_router)


@app.get("/api/healthcheck.json", response_model=Dict[str, Any])
async def healthcheck() -> Dict[str, Any]:
    return {"status": "healthy", "timestamp": int(__import__("time").time())}
