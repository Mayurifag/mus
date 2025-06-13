from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from typing import Dict, Any
from contextlib import asynccontextmanager
import asyncio
import logging

from src.mus.infrastructure.api.middleware.auth import AuthMiddleware, auth_router
from src.mus.infrastructure.api.routers import (
    player_router,
    track_router,
)
from src.mus.infrastructure.api.sse_handler import router as sse_router
from src.mus.infrastructure.database import (
    create_db_and_tables,
    async_session_factory,
)
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.infrastructure.tasks.background_scanner import PeriodicScanner
from src.mus.config import settings

logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    await create_db_and_tables()
    settings.COVERS_DIR_PATH.mkdir(parents=True, exist_ok=True)

    file_system_scanner = FileSystemScanner()
    cover_processor = CoverProcessor(covers_dir_path=settings.COVERS_DIR_PATH)
    scan_use_case = ScanTracksUseCase(
        session_factory=async_session_factory,
        file_system_scanner=file_system_scanner,
        cover_processor=cover_processor,
    )

    scanner = PeriodicScanner(
        scan_use_case=scan_use_case,
        scan_interval_seconds=settings.SCAN_INTERVAL_SECONDS,
    )
    app.state.periodic_scanner = scanner
    asyncio.create_task(scanner.start())
    yield
    logger.info("Application lifespan shutdown...")
    if hasattr(app.state, "periodic_scanner") and app.state.periodic_scanner:
        logger.info("Stopping periodic scanner...")
        await app.state.periodic_scanner.stop()
        logger.info("Periodic scanner stopped.")
    else:
        logger.info("No periodic scanner found in app.state or already stopped.")


app = FastAPI(
    title="Mus API",
    description="Music streaming API for Mus project",
    lifespan=lifespan,
)

# Configure CORS only if not in production
if settings.APP_ENV != "production":
    app.add_middleware(
        CORSMiddleware,
        allow_origins=settings.CORS_ALLOWED_ORIGINS,
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

# Add authentication middleware
app.add_middleware(AuthMiddleware)

# Include routers
app.include_router(auth_router)
app.include_router(player_router.router)
app.include_router(track_router.router)
app.include_router(sse_router)


# The following routes were in the provided main.py context, keeping them.
@app.get("/api", response_model=Dict[str, Any])
async def read_root() -> Dict[str, Any]:
    return {"status": "ok", "message": "Mus backend is running"}


logger.info(f"Music directory configured: {settings.MUSIC_DIR_PATH}")
logger.info(f"Covers will be stored in: {settings.COVERS_DIR_PATH}")
logger.info(f"Log level set to: {settings.LOG_LEVEL}")
logger.info(f"Scan interval set to: {settings.SCAN_INTERVAL_SECONDS} seconds")
