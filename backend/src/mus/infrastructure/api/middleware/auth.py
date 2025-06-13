import hmac
from datetime import datetime, timedelta, UTC
from typing import Optional

from fastapi import Request, Response, APIRouter, status
from fastapi.responses import RedirectResponse, JSONResponse
from starlette.middleware.base import BaseHTTPMiddleware
from jose import jwt, JWTError

from src.mus.config import settings

ALGORITHM = "HS256"
AUTH_COOKIE_NAME = "mus_auth_token"

auth_router = APIRouter(prefix="/api/v1/auth", tags=["authentication"])


def create_access_token(data: dict, expires_delta: Optional[timedelta] = None) -> str:
    to_encode = data.copy()
    if expires_delta:
        expire = datetime.now(UTC) + expires_delta
    else:
        days = 365 if settings.APP_ENV == "production" else 30
        expire = datetime.now(UTC) + timedelta(days=days)
    to_encode.update({"exp": expire, "iat": datetime.now(UTC)})
    if not settings.SECRET_KEY:
        raise ValueError("SECRET_KEY is not configured")
    return jwt.encode(to_encode, settings.SECRET_KEY, algorithm=ALGORITHM)


@auth_router.get("/login-by-secret/{secret_key}", response_model=None)
async def login_by_secret(secret_key: str, request: Request):
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

    access_token = create_access_token({})

    frontend_url = (
        "http://localhost:5173"
        if settings.APP_ENV != "production"
        else f"{request.url.scheme}://{request.url.netloc}"
    )

    response = RedirectResponse(url=frontend_url, status_code=status.HTTP_303_SEE_OTHER)

    days = 365 if settings.APP_ENV == "production" else 30
    max_age = days * 24 * 60 * 60

    response.set_cookie(
        key=AUTH_COOKIE_NAME,
        value=access_token,
        httponly=True,
        max_age=max_age,
        secure=settings.APP_ENV == "production",
        samesite="strict" if settings.APP_ENV == "production" else "lax",
    )

    return response


@auth_router.get("/auth-status")
async def auth_status(request: Request) -> dict:
    if not settings.SECRET_KEY:
        return {"auth_enabled": False, "authenticated": False}

    token = request.cookies.get(AUTH_COOKIE_NAME)
    if not token:
        return {"auth_enabled": True, "authenticated": False}

    try:
        payload = jwt.decode(token, settings.SECRET_KEY, algorithms=[ALGORITHM])
        exp = payload.get("exp")
        if exp and datetime.fromtimestamp(exp, tz=UTC) < datetime.now(UTC):
            return {"auth_enabled": True, "authenticated": False}
        return {"auth_enabled": True, "authenticated": True}
    except JWTError:
        return {"auth_enabled": True, "authenticated": False}


class AuthMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request: Request, call_next) -> Response:
        if not settings.SECRET_KEY:
            return await call_next(request)

        public_paths = ["/api/v1/auth/", "/api/v1/events/"]

        if any(request.url.path.startswith(path) for path in public_paths):
            return await call_next(request)

        token = request.cookies.get(AUTH_COOKIE_NAME)
        if not token:
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content={"detail": "Authentication required"},
            )

        try:
            payload = jwt.decode(token, settings.SECRET_KEY, algorithms=[ALGORITHM])
            exp = payload.get("exp")
            if exp and datetime.fromtimestamp(exp, tz=UTC) < datetime.now(UTC):
                return JSONResponse(
                    status_code=status.HTTP_401_UNAUTHORIZED,
                    content={"detail": "Token expired"},
                )
        except JWTError:
            return JSONResponse(
                status_code=status.HTTP_401_UNAUTHORIZED,
                content={"detail": "Invalid token"},
            )

        return await call_next(request)
