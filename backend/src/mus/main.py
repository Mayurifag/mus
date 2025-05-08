from contextlib import asynccontextmanager
from fastapi import FastAPI, Depends
from typing import Dict, Any

from src.mus.infrastructure.api.auth import router as auth_router
from src.mus.infrastructure.api.dependencies import get_current_user
from src.mus.infrastructure.api.routers.player_router import router as player_router
from src.mus.infrastructure.api.routers.track_router import router as track_router
from src.mus.infrastructure.database import create_db_and_tables


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Initialize database on startup
    await create_db_and_tables()
    yield


app = FastAPI(
    title="Mus API",
    description="Music streaming API for Mus project",
    lifespan=lifespan,
)

# Include routers
app.include_router(auth_router)
app.include_router(player_router)
app.include_router(track_router)


@app.get("/", response_model=Dict[str, Any])
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
