from fastapi import APIRouter

from src.mus.config import settings

router = APIRouter(prefix="/api/v1/auth", tags=["authentication"])


@router.get("/qr-code-url")
async def get_qr_code_url() -> dict:
    if not settings.SECRET_KEY:
        return {"url": ""}

    magic_link_url = f"/login?token={settings.SECRET_KEY}"
    return {"url": magic_link_url}
