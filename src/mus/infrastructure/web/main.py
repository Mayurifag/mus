from contextlib import asynccontextmanager

import structlog
from fastapi import FastAPI, Request
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

from mus.config import get_music_dir
from mus.dependencies import get_scan_tracks_use_case, get_search_tracks_use_case
from mus.infrastructure.logging_config import setup_logging

# Setup logging
setup_logging()
log = structlog.get_logger()


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup: Run initial scan
    scan_use_case = get_scan_tracks_use_case()
    await scan_use_case.execute(get_music_dir())
    yield
    # Shutdown: Clean up resources if needed
    pass


app = FastAPI(title="mus", lifespan=lifespan)

# Mount static files
app.mount(
    "/static", StaticFiles(directory="src/mus/infrastructure/web/static"), name="static"
)

# Setup templates
templates = Jinja2Templates(directory="src/mus/infrastructure/web/templates")


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
    scan_use_case = get_scan_tracks_use_case()
    await scan_use_case.execute(get_music_dir())
    return HTMLResponse("Scan completed", status_code=200)
