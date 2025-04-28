import structlog
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates

from mus.infrastructure.logging_config import setup_logging

# Setup logging
setup_logging()
log = structlog.get_logger()

app = FastAPI(title="mus")

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
    return templates.TemplateResponse("index.html", {"request": request})
