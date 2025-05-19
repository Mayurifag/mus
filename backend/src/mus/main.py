from fastapi import FastAPI, Depends
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from typing import Dict, Any
import os
from contextlib import asynccontextmanager
import asyncio
import logging
from pathlib import Path

from src.mus.infrastructure.api.auth import router as auth_router
from src.mus.infrastructure.api.dependencies import get_current_user
from src.mus.infrastructure.api.routers import (
    player_router,
    track_router,
    album_router,
    artist_router,
    playlist_router,
)
from src.mus.infrastructure.api.sse_handler import router as sse_router
from src.mus.infrastructure.database import (
    engine,
    SQLModel,
    create_db_and_tables,
    async_session_factory,
)
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.infrastructure.tasks.background_scanner import PeriodicScanner
from src.mus.config import settings

# Configure logging
logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info("Application lifespan startup...")
    # Drop and create tables only if in development/testing, or based on a setting
    if settings.APP_ENV != "production":  # Example condition
        logger.warning(f"DROPPING AND RECREATING TABLES in {settings.APP_ENV} mode.")
        async with engine.begin() as conn:
            await conn.run_sync(SQLModel.metadata.drop_all)
            await conn.run_sync(SQLModel.metadata.create_all)
    # Always ensure tables and cover dir exist
    await create_db_and_tables()
    settings.COVERS_DIR_PATH.mkdir(parents=True, exist_ok=True)

    file_system_scanner = FileSystemScanner()
    cover_processor = CoverProcessor(
        covers_dir_path=settings.COVERS_DIR_PATH
    )  # Explicitly named arg

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
    logger.info("Periodic scanner scheduled to start.")
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
if os.getenv("APP_ENV") != "production":
    app.add_middleware(
        CORSMiddleware,
        allow_origins=settings.CORS_ALLOWED_ORIGINS,
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

# Include routers
app.include_router(auth_router)
app.include_router(player_router.router)
app.include_router(track_router.router)
app.include_router(album_router.router)
app.include_router(artist_router.router)
app.include_router(playlist_router.router)
app.include_router(sse_router)

logger.info(
    f"Application configured with CORS origins: {settings.CORS_ALLOWED_ORIGINS}"
)

# This mounts what's in settings.STATIC_DIR_PATH at /static URL path
if settings.STATIC_DIR_PATH.exists() and settings.STATIC_DIR_PATH.is_dir():
    app.mount(
        "/static",
        StaticFiles(directory=str(settings.STATIC_DIR_PATH)),
        name="static_assets",
    )
    logger.info(f"Serving static files from: {settings.STATIC_DIR_PATH} at /static")
else:
    logger.warning(
        f"Static directory {settings.STATIC_DIR_PATH} not found, not mounting /static."
    )


# The following routes were in the provided main.py context, keeping them.
@app.get("/api", response_model=Dict[str, Any])
async def read_root() -> Dict[str, Any]:
    return {"status": "ok", "message": "Mus backend is running"}


@app.get("/api/v1/me", response_model=Dict[str, Any])
async def get_current_user_info(
    current_user: Dict[str, Any] = Depends(get_current_user),
) -> Dict[str, Any]:
    """
    Get current authenticated user info (protected endpoint example)
    """
    return {"user": current_user, "authenticated": True}


# Frontend static file serving from original main.py structure
# This path needs to be correct relative to where the app runs, or use an absolute path from settings.
# Assuming PROJECT_ROOT from settings is the actual project root.
# frontend_static_path = settings.PROJECT_ROOT / "frontend" / ".svelte-kit" / "output" / "client"
# A more robust way would be to have FRONTEND_BUILD_DIR in settings.
# For now, using a placeholder that matches common deployment patterns or local dev.
_default_frontend_path = (
    settings.PROJECT_ROOT / "frontend" / "build"
)  # Common for SvelteKit adapter-static
_alternative_frontend_path = (
    settings.PROJECT_ROOT / "static"
)  # If frontend build is copied to a top-level static

frontend_serving_path: Path | None = None
if _default_frontend_path.exists() and _default_frontend_path.is_dir():
    frontend_serving_path = _default_frontend_path
elif _alternative_frontend_path.exists() and _alternative_frontend_path.is_dir():
    frontend_serving_path = _alternative_frontend_path

if frontend_serving_path:
    logger.info(
        f"Attempting to serve frontend static files from: {frontend_serving_path}"
    )
    app.mount(
        "/",
        StaticFiles(directory=str(frontend_serving_path), html=True),
        name="frontend",
    )
else:
    logger.warning(
        "Frontend static directory not found at common locations, not mounting SPA."
    )

logger.info(f"Music directories configured: {settings.MUSIC_DIRECTORIES_LIST}")
logger.info(f"Covers will be stored in: {settings.COVERS_DIR_PATH}")
logger.info(f"Log level set to: {settings.LOG_LEVEL}")
logger.info(f"Scan interval set to: {settings.SCAN_INTERVAL_SECONDS} seconds")
