import pytest
from fastapi.testclient import TestClient
from httpx import Response
from typing import Generator
from jose import jwt
import sys
from pathlib import Path

# Add the src directory to the Python path
sys.path.append(str(Path(__file__).parent.parent))

from src.mus.main import app
from src.mus.config import settings
from src.mus.infrastructure.api.auth import ALGORITHM, COOKIE_NAME


@pytest.fixture
def client() -> Generator[TestClient, None, None]:
    """
    Create a test client for the app
    """
    with TestClient(app, follow_redirects=False) as test_client:
        yield test_client


def test_login_via_secret_valid(client: TestClient) -> None:
    """
    Test login with valid secret key
    """
    response: Response = client.get(
        f"/api/v1/auth/login-via-secret/{settings.SECRET_KEY}"
    )

    # Should redirect to root
    assert response.status_code == 303
    assert response.headers["location"] == "/"

    # Should set auth cookie
    cookies = response.cookies
    assert COOKIE_NAME in cookies

    # Verify token is valid
    token = cookies[COOKIE_NAME]
    payload = jwt.decode(token, settings.SECRET_KEY, algorithms=[ALGORITHM])
    assert payload["sub"] == "web-user"
    assert "exp" in payload
    assert "iat" in payload


def test_login_via_secret_invalid(client: TestClient) -> None:
    """
    Test login with invalid secret key
    """
    response: Response = client.get("/api/v1/auth/login-via-secret/invalid-key")
    assert response.status_code == 401


def test_protected_route_authenticated(client: TestClient) -> None:
    """
    Test protected route with valid authentication
    """
    # First login to get auth cookie
    client.get(f"/api/v1/auth/login-via-secret/{settings.SECRET_KEY}")

    # Then access protected route
    response: Response = client.get("/api/v1/me")
    assert response.status_code == 200
    data = response.json()
    assert data["authenticated"] is True
    assert "user" in data


def test_protected_route_unauthenticated(client: TestClient) -> None:
    """
    Test protected route without authentication
    """
    response: Response = client.get("/api/v1/me")
    assert response.status_code == 401
