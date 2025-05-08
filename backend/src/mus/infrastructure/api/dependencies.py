from fastapi import Cookie, HTTPException, status
from jose import JWTError, jwt
from typing import Optional, Dict, Any
from datetime import datetime, UTC

from src.mus.config import settings
from src.mus.infrastructure.api.auth import ALGORITHM, COOKIE_NAME


async def get_current_user(
    token: Optional[str] = Cookie(None, alias=COOKIE_NAME),
) -> Dict[str, Any]:
    """
    Get current user from JWT token in cookie
    """
    credentials_exception = HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )

    if token is None:
        raise credentials_exception

    try:
        payload = jwt.decode(token, settings.SECRET_KEY, algorithms=[ALGORITHM])

        # Validate that token hasn't expired
        exp = payload.get("exp")
        if exp and datetime.fromtimestamp(exp, tz=UTC) < datetime.now(UTC):
            raise credentials_exception

        return payload
    except JWTError:
        raise credentials_exception
