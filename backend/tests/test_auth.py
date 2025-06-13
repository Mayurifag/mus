import pytest
from fastapi.testclient import TestClient
from httpx import Response
from typing import Generator
from jose import jwt
from unittest.mock import patch

from src.mus.main import app
from src.mus.config import settings
from src.mus.infrastructure.api.middleware.auth import ALGORITHM, AUTH_COOKIE_NAME


@pytest.fixture
def client() -> Generator[TestClient, None, None]:
    with TestClient(app, follow_redirects=False) as test_client:
        yield test_client


def test_login_by_secret_valid_with_secret_key(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        response: Response = client.get("/api/v1/auth/login-by-secret/test-secret-key")

        assert response.status_code == 303
        assert response.headers["location"] == "http://localhost:5173"

        cookies = response.cookies
        assert AUTH_COOKIE_NAME in cookies

        token = cookies[AUTH_COOKIE_NAME]
        payload = jwt.decode(token, "test-secret-key", algorithms=[ALGORITHM])
        assert "exp" in payload
        assert "iat" in payload


def test_login_by_secret_invalid_key(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        response: Response = client.get("/api/v1/auth/login-by-secret/invalid-key")
        assert response.status_code == 401


def test_login_by_secret_no_secret_configured(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", None):
        response: Response = client.get("/api/v1/auth/login-by-secret/any-key")
        assert response.status_code == 404


def test_auth_status_no_secret_configured(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", None):
        response: Response = client.get("/api/v1/auth/auth-status")
        assert response.status_code == 200
        data = response.json()
        assert data["auth_enabled"] is False
        assert data["authenticated"] is False


def test_auth_status_with_secret_unauthenticated(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        response: Response = client.get("/api/v1/auth/auth-status")
        assert response.status_code == 200
        data = response.json()
        assert data["auth_enabled"] is True
        assert data["authenticated"] is False


def test_auth_status_with_secret_authenticated(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        client.get("/api/v1/auth/login-by-secret/test-secret-key")
        response: Response = client.get("/api/v1/auth/auth-status")
        assert response.status_code == 200
        data = response.json()
        assert data["auth_enabled"] is True
        assert data["authenticated"] is True


def test_protected_route_authenticated(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        client.get("/api/v1/auth/login-by-secret/test-secret-key")
        response: Response = client.get("/api")
        assert response.status_code == 200


def test_protected_route_unauthenticated(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        response: Response = client.get("/api")
        assert response.status_code == 401


def test_protected_route_no_auth_configured(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", None):
        response: Response = client.get("/api")
        assert response.status_code == 200
