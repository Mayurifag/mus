from fastapi import APIRouter, HTTPException, status
from fastapi.responses import RedirectResponse
from datetime import datetime, timedelta, UTC
from typing import Dict, Optional, Any
from jose import jwt

from src.mus.config import settings

ALGORITHM = "HS256"
ACCESS_TOKEN_EXPIRE_MINUTES = 1440  # 24 hours
COOKIE_NAME = "mus_auth_token"

router = APIRouter(prefix="/api/v1/auth", tags=["authentication"])


def create_access_token(
    data: Dict[str, Any], expires_delta: Optional[timedelta] = None
) -> str:
    to_encode = data.copy()

    if expires_delta:
        expire = datetime.now(UTC) + expires_delta
    else:
        expire = datetime.now(UTC) + timedelta(minutes=ACCESS_TOKEN_EXPIRE_MINUTES)

    to_encode.update({"exp": expire, "iat": datetime.now(UTC)})
    return jwt.encode(to_encode, settings.SECRET_KEY, algorithm=ALGORITHM)


@router.get("/login-via-secret/{secret_key}")
async def login_via_secret(secret_key: str) -> RedirectResponse:
    if secret_key != settings.SECRET_KEY:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid authentication credentials",
        )

    token_data = {"sub": "web-user"}
    access_token = create_access_token(token_data)

    response = RedirectResponse(url="/", status_code=status.HTTP_303_SEE_OTHER)
    response.set_cookie(
        key=COOKIE_NAME,
        value=access_token,
        httponly=True,
        max_age=ACCESS_TOKEN_EXPIRE_MINUTES * 60,
        samesite="lax",
        secure=False,
    )

    return response
