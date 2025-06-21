from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from typing import Dict, Any
from contextlib import asynccontextmanager
import logging


from src.mus.infrastructure.api.routers import (
    auth_router,
    player_router,
    track_router,
)
from src.mus.infrastructure.api.sse_handler import router as sse_router
from src.mus.infrastructure.database import create_db_and_tables

from src.mus.config import settings

logging.basicConfig(
    level=settings.LOG_LEVEL.upper(), format="%(levelname)s:     %(name)s - %(message)s"
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    settings.DATABASE_PATH.parent.mkdir(parents=True, exist_ok=True)
    settings.COVERS_DIR_PATH.mkdir(parents=True, exist_ok=True)
    settings.MUSIC_DIR_PATH.mkdir(parents=True, exist_ok=True)
    await create_db_and_tables()
    yield


app = FastAPI(
    title="Mus API",
    description="Music streaming API for Mus project",
    lifespan=lifespan,
)

# Configure CORS only if not in production
if settings.APP_ENV != "production":
    app.add_middleware(
        CORSMiddleware,
        allow_origins=[settings.FRONTEND_ORIGIN],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

# Include routers
app.include_router(auth_router.router)
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
