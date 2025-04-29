from contextlib import asynccontextmanager
from datetime import datetime

import structlog
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import FileResponse, HTMLResponse, JSONResponse, Response
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

APP_START_TIME = int(datetime.now().timestamp())


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


app = FastAPI(title="mus", lifespan=lifespan)
app.mount(
    "/static", StaticFiles(directory="src/mus/infrastructure/web/static"), name="static"
)
templates = Jinja2Templates(directory="src/mus/infrastructure/web/templates")
templates.env.filters["datetime"] = lambda ts: datetime.fromtimestamp(ts).strftime(
    "%Y-%m-%d %H:%M"
)


def render_template(request: Request, template_name: str, **context) -> Response:
    """Helper function to render templates with consistent context."""
    return templates.TemplateResponse(
        request, template_name, {"request": request, **context}
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
    return render_template(request, "index.html", startup_ts=APP_START_TIME)


@app.get("/tracks")
async def get_tracks(request: Request):
    """Get all tracks."""
    log.info("Getting all tracks")
    search_use_case = get_search_tracks_use_case()
    tracks = await search_use_case.get_all()
    return render_template(request, "_track_list.html", tracks=tracks)


@app.post("/scan")
async def scan_tracks():
    """Trigger track scanning."""
    log.info("Manual scan triggered")
    repository = get_track_repository()
    log.info("Clearing existing tracks before scan")
    await repository.clear_all_tracks()
    scan_use_case = get_scan_tracks_use_case()
    await scan_use_case.execute(get_music_dir())
    return HTMLResponse(
        "Scan completed", status_code=200, headers={"HX-Trigger": "refreshTrackList"}
    )


@app.get("/stream/{track_id:int}")
async def stream_audio_by_id(track_id: int):
    """Stream audio file by track ID."""
    track = await get_track_repository().get_by_id(track_id)

    if not track or not track.file_path.exists():
        raise HTTPException(status_code=404)

    return FileResponse(
        str(track.file_path),
        media_type={
            ".wav": "audio/wav",
            ".wave": "audio/wav",
            ".ogg": "audio/ogg",
            ".oga": "audio/ogg",
            ".flac": "audio/flac",
        }.get(track.file_path.suffix.lower(), "audio/mpeg"),
        filename=track.file_path.name,
    )
