import hmac
import re
from typing import Optional

from fastapi import Request, Response, APIRouter, status
from fastapi.responses import RedirectResponse, JSONResponse
from starlette.middleware.base import BaseHTTPMiddleware

from src.mus.config import settings

auth_router = APIRouter(prefix="/api/v1/auth", tags=["authentication"])

PUBLIC_PATH_REGEX = re.compile(
    r"^(?:"
    r"/api/v1/auth/.*|"  # Authentication endpoints
    r"/api/v1/events/.*|"  # Server-sent events endpoints
    r"/api/v1/tracks/\d+/stream$|"  # Track streaming endpoints
    r"/api/v1/tracks/\d+/covers/.*|"  # Track cover image endpoints
    r"/api/v1/player/state$"  # Player state endpoints (for beacons)
    r")$"
)


def _extract_bearer_token(request: Request) -> Optional[str]:
    auth_header = request.headers.get("authorization")
    if auth_header and auth_header.startswith("Bearer "):
        return auth_header[7:]
    return None


def _validate_bearer_token(token: Optional[str]) -> bool:
    if not token or not settings.SECRET_KEY:
        return False
    return hmac.compare_digest(token, settings.SECRET_KEY)


@auth_router.get("/login-by-secret/{secret_key}", response_model=None)
async def login_by_secret(secret_key: str):
    if not settings.SECRET_KEY:
        return JSONResponse(
            status_code=status.HTTP_404_NOT_FOUND,
            content={"detail": "Authentication not configured"},
        )

    if not hmac.compare_digest(secret_key, settings.SECRET_KEY):
        return JSONResponse(
            status_code=status.HTTP_401_UNAUTHORIZED,
            content={"detail": "Invalid authentication credentials"},
        )

    redirect_url = f"{settings.FRONTEND_ORIGIN}/auth/callback?token={secret_key}"
    return RedirectResponse(url=redirect_url, status_code=status.HTTP_303_SEE_OTHER)


@auth_router.get("/auth-status")
async def auth_status(request: Request) -> dict:
    if not settings.SECRET_KEY:
        return {"auth_enabled": False, "authenticated": False}

    token = _extract_bearer_token(request)
    return {"auth_enabled": True, "authenticated": _validate_bearer_token(token)}


def _is_public_path(path: str) -> bool:
    return bool(PUBLIC_PATH_REGEX.match(path))


class AuthMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next) -> Response:
        if not settings.SECRET_KEY:
            return await call_next(request)

        # Allow CORS preflight requests to pass through
        if request.method == "OPTIONS":
            return await call_next(request)

        if _is_public_path(request.url.path):
            return await call_next(request)

        token = _extract_bearer_token(request)
        if not _validate_bearer_token(token):
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content={"detail": "Authentication required"},
            )

        return await call_next(request)
