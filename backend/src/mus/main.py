import logging
from contextlib import asynccontextmanager
from typing import Any, Dict

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from src.mus.config import settings
from src.mus.infrastructure.api.routers import (
    auth_router,
    player_router,
    track_router,
)
from src.mus.infrastructure.api.sse_handler import router as sse_router
from src.mus.service.file_watcher_manager import FileWatcherManager
from src.mus.service.scanner_service import InitialScanner

logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(_: FastAPI):
    await (await InitialScanner.create_default()).scan()

    file_watcher_manager = FileWatcherManager()
    await file_watcher_manager.start()

    try:
        yield
    finally:
        file_watcher_manager.stop()


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
app.include_router(sse_router)


@app.get("/api", response_model=Dict[str, Any])
async def read_root() -> Dict[str, Any]:
    return {"status": "ok", "message": "mus backend is running"}
