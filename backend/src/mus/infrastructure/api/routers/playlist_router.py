from fastapi import APIRouter

router = APIRouter(
    prefix="/api/v1/playlists",
    tags=["playlists"],
)
