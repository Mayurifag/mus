from contextlib import asynccontextmanager
from fastapi import FastAPI, Depends
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from typing import Dict, Any
import os
from pathlib import Path
from dotenv import load_dotenv
import shutil
import asyncio

from src.mus.infrastructure.api.auth import router as auth_router
from src.mus.infrastructure.api.dependencies import get_current_user
from src.mus.infrastructure.api.routers.player_router import router as player_router
from src.mus.infrastructure.api.routers.track_router import router as track_router
from src.mus.infrastructure.api.routers.scan_router import router as scan_router
from src.mus.infrastructure.database import engine, SQLModel, get_session_generator
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase

load_dotenv()


def run_scan():
    async def inner():
        async for session in get_session_generator():
            repo = SQLiteTrackRepository(session)
            scanner = FileSystemScanner()
            processor = CoverProcessor()
            use_case = ScanTracksUseCase(repo, scanner, processor)
            await use_case.scan_directory()
            break

    return inner


@asynccontextmanager
async def lifespan(app: FastAPI):
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.drop_all)
        await conn.run_sync(SQLModel.metadata.create_all)
    covers_dir = Path("./data/covers")
    shutil.rmtree(covers_dir, ignore_errors=True)
    os.makedirs(covers_dir, exist_ok=True)
    asyncio.create_task(run_scan()())
    yield


app = FastAPI(
    title="Mus API",
    description="Music streaming API for Mus project",
    lifespan=lifespan,
)

# Configure CORS only if not in production
if os.getenv("APP_ENV") != "production":
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["http://localhost:5173"],  # SvelteKit dev server
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

# Include routers
app.include_router(auth_router)
app.include_router(player_router)
app.include_router(track_router)
app.include_router(scan_router)


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


static_path = "/app/static_root"
if not os.path.exists(static_path):
    static_path = "static"

if os.path.exists(static_path):
    app.mount("/", StaticFiles(directory=static_path, html=True), name="frontend")
