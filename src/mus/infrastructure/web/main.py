import hashlib
from contextlib import asynccontextmanager
from datetime import datetime

import structlog
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import FileResponse, HTMLResponse, JSONResponse, Response
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

from mus.config import get_covers_dir
from mus.dependencies import (
    get_full_scan_interactor,
    get_initial_state_service,
    get_track_repository,
)
from mus.infrastructure.logging_config import setup_logging
from mus.infrastructure.web.api.state_router import router as state_router

setup_logging()
log = structlog.get_logger()

APP_START_TIME = int(datetime.now().timestamp())


@asynccontextmanager
async def lifespan(app: FastAPI):
    try:
        interactor = get_full_scan_interactor()
        await interactor.execute()
    except Exception as e:
        log.exception("Failed to perform initial scan", exc_info=e)
        raise
    yield


app = FastAPI(title="mus", lifespan=lifespan)
app.mount(
    "/static", StaticFiles(directory="src/mus/infrastructure/web/static"), name="static"
)
app.include_router(state_router)
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
    initial_state_service = get_initial_state_service()
    initial_state = await initial_state_service.get_initial_state()
    return render_template(
        request, "index.html", startup_ts=APP_START_TIME, initial_state=initial_state
    )


@app.get("/tracks")
async def get_tracks(request: Request):
    """Get all tracks."""
    log.info("Getting all tracks")
    initial_state_service = get_initial_state_service()
    initial_state = await initial_state_service.get_initial_state()
    return render_template(request, "_track_list.html", tracks=initial_state["tracks"])


@app.post("/scan")
async def scan_tracks():
    """Trigger track scanning."""
    log.info("Manual scan triggered")
    interactor = get_full_scan_interactor()
    await interactor.execute()
    return HTMLResponse(
        "Scan completed", status_code=200, headers={"HX-Trigger": "refreshTrackList"}
    )


@app.get("/stream/{track_id:int}")
async def stream_audio_by_id(track_id: int):
    """Stream audio file by track ID."""
    track = await get_track_repository().get_by_id(track_id)

    if not track or not track.file_path.exists():
        raise HTTPException(status_code=404)

    file_stat = track.file_path.stat()
    etag = hashlib.md5(
        f"{track_id}:{file_stat.st_size}:{file_stat.st_mtime}".encode()
    ).hexdigest()

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
        headers={
            "Cache-Control": "public, max-age=604800",
            "ETag": f'"{etag}"',
        },
    )


@app.get("/covers/{size}/{track_id:int}.webp")
async def get_cover(size: str, track_id: int):
    """Serve cover art images.

    Args:
        size: Size of the cover image ('small' or 'medium')
        track_id: ID of the track

    Returns:
        FileResponse: The cover image file
    """
    if size not in ["small", "medium"]:
        raise HTTPException(status_code=400, detail="Invalid size")

    cover_path = get_covers_dir() / f"{track_id}_{size}.webp"
    if not cover_path.exists():
        # Return placeholder image
        return FileResponse(
            "src/mus/infrastructure/web/static/images/placeholder.svg",
            media_type="image/svg+xml",
        )

    return FileResponse(str(cover_path), media_type="image/webp")
