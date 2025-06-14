import hmac
from typing import Optional
from urllib.parse import urlparse

from fastapi import Request, Response, APIRouter, status
from fastapi.responses import RedirectResponse, JSONResponse
from starlette.middleware.base import BaseHTTPMiddleware
from jose import jwt, JWTError

from src.mus.config import settings

ALGORITHM = "HS256"
AUTH_COOKIE_NAME = "mus_auth_token"

auth_router = APIRouter(prefix="/api/v1/auth", tags=["authentication"])


def _get_cookie_domain() -> Optional[str]:
    if settings.APP_ENV in ("production", "test") or not settings.FRONTEND_ORIGIN:
        return None

    parsed = urlparse(settings.FRONTEND_ORIGIN)
    return parsed.hostname


def _validate_token(token: Optional[str]) -> bool:
    if not token or not settings.SECRET_KEY:
        return False

    try:
        jwt.decode(token, settings.SECRET_KEY, algorithms=[ALGORITHM])
        return True
    except JWTError:
        return False


def create_access_token() -> str:
    if not settings.SECRET_KEY:
        raise ValueError("SECRET_KEY is not configured")

    return jwt.encode({}, settings.SECRET_KEY, algorithm=ALGORITHM)


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

    access_token = create_access_token()
    response = RedirectResponse(
        url=settings.FRONTEND_ORIGIN, status_code=status.HTTP_303_SEE_OTHER
    )

    is_production = settings.APP_ENV == "production"
    response.set_cookie(
        key=AUTH_COOKIE_NAME,
        value=access_token,
        httponly=True,
        secure=is_production,
        samesite="strict" if is_production else "lax",
        path="/",
        domain=_get_cookie_domain(),
    )

    return response


@auth_router.get("/auth-status")
async def auth_status(request: Request) -> dict:
    if not settings.SECRET_KEY:
        return {"auth_enabled": False, "authenticated": False}

    token = request.cookies.get(AUTH_COOKIE_NAME)
    return {"auth_enabled": True, "authenticated": _validate_token(token)}


def _is_ssr_request(request: Request) -> bool:
    user_agent = request.headers.get("user-agent", "")
    proxy_headers = [
        "x-forwarded-for",
        "x-real-ip",
        "x-forwarded-proto",
        "x-forwarded-host",
    ]
    has_proxy_headers = any(request.headers.get(header) for header in proxy_headers)
    return user_agent == "node" and not has_proxy_headers


def _is_public_path(path: str) -> bool:
    public_paths = ["/api/v1/auth/", "/api/v1/events/"]
    return any(path.startswith(public_path) for public_path in public_paths)


class AuthMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next) -> Response:
        if not settings.SECRET_KEY:
            return await call_next(request)

        if _is_ssr_request(request):
            return await call_next(request)

        if _is_public_path(request.url.path):
            return await call_next(request)

        token = request.cookies.get(AUTH_COOKIE_NAME)
        if not _validate_token(token):
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content={"detail": "Authentication required"},
            )

        return await call_next(request)
