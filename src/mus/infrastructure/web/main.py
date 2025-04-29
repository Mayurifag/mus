from contextlib import asynccontextmanager
from datetime import datetime
import os
from pathlib import Path

import structlog
from fastapi import FastAPI, Request, HTTPException
from fastapi.responses import HTMLResponse, JSONResponse, FileResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

from mus.config import get_music_dir
from mus.dependencies import (
    get_scan_tracks_use_case,
    get_search_tracks_use_case,
    get_track_repository,
)
from mus.infrastructure.logging_config import setup_logging

setup_logging()
log = structlog.get_logger()


@asynccontextmanager
async def lifespan(app: FastAPI):
    try:
        repository = get_track_repository()
        log.info("Clearing existing tracks before initial scan")
        await repository.clear_all_tracks()
    except Exception as e:
        log.exception("Failed to clear tracks before initial scan", exc_info=e)

    scan_use_case = get_scan_tracks_use_case()
    await scan_use_case.execute(get_music_dir())
    yield
    pass


app = FastAPI(title="mus", lifespan=lifespan)
app.mount(
    "/static", StaticFiles(directory="src/mus/infrastructure/web/static"), name="static"
)
templates = Jinja2Templates(directory="src/mus/infrastructure/web/templates")
templates.env.filters["datetime"] = lambda ts: datetime.fromtimestamp(ts).strftime(
    "%Y-%m-%d %H:%M"
)

@app.exception_handler(Exception)
async def global_exception_handler(request: Request, exc: Exception):
    """Global exception handler for unhandled exceptions."""
    log.exception("Unhandled error", exc_info=exc)
    return JSONResponse(
        status_code=500,
        content={"detail": "Internal Server Error"},
    )


@app.get("/")
async def root(request: Request):
    """Root route handler."""
    log.info("Root route accessed")
    return templates.TemplateResponse(request, "index.html", {"request": request})


@app.get("/tracks")
async def get_tracks(request: Request):
    """Get all tracks."""
    log.info("Getting all tracks")
    search_use_case = get_search_tracks_use_case()
    tracks = await search_use_case.get_all()
    return templates.TemplateResponse(
        request,
        "_track_list.html",
        {"request": request, "tracks": tracks},
    )


@app.post("/scan")
async def scan_tracks():
    """Trigger track scanning."""
    log.info("Manual scan triggered")
    repository = get_track_repository()
    log.info("Clearing existing tracks before scan")
    await repository.clear_all_tracks()
    scan_use_case = get_scan_tracks_use_case()
    await scan_use_case.execute(get_music_dir())
    return HTMLResponse("Scan completed", status_code=200)


@app.get("/stream/{file_path:path}")
async def stream_audio(file_path: str):
    """Stream audio file with support for range requests."""
    music_dir = Path(get_music_dir())
    requested_path = Path(file_path)

    # Validate path is within music directory
    try:
        absolute_path = (music_dir / requested_path).resolve()
        if not str(absolute_path).startswith(str(music_dir)):
            raise HTTPException(status_code=403, detail="Access denied")
    except Exception as e:
        raise HTTPException(status_code=404, detail="Invalid file path")

    if not absolute_path.exists() or not absolute_path.is_file():
        raise HTTPException(status_code=404, detail="File not found")

    # Determine media type based on extension
    media_type = "audio/mpeg"  # Default to MP3
    if absolute_path.suffix.lower() in [".wav", ".wave"]:
        media_type = "audio/wav"
    elif absolute_path.suffix.lower() in [".ogg", ".oga"]:
        media_type = "audio/ogg"
    elif absolute_path.suffix.lower() in [".flac"]:
        media_type = "audio/flac"

    return FileResponse(
        str(absolute_path),
        media_type=media_type,
        filename=absolute_path.name
    )
