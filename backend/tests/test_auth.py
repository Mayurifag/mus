import pytest
from fastapi.testclient import TestClient
from httpx import Response
from typing import Generator
from unittest.mock import patch

from src.mus.main import app
from src.mus.config import settings


@pytest.fixture
def client() -> Generator[TestClient, None, None]:
    with TestClient(app, follow_redirects=False) as test_client:
        yield test_client


def test_login_by_secret_valid_with_secret_key(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        response: Response = client.get("/api/v1/auth/login-by-secret/test-secret-key")

        assert response.status_code == 303
        expected_location = "http://localhost:5173/auth/callback?token=test-secret-key"
        assert response.headers["location"] == expected_location


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
        headers = {"Authorization": "Bearer test-secret-key"}
        response: Response = client.get("/api/v1/auth/auth-status", headers=headers)
        assert response.status_code == 200
        data = response.json()
        assert data["auth_enabled"] is True
        assert data["authenticated"] is True


def test_protected_route_authenticated(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        headers = {"Authorization": "Bearer test-secret-key"}
        response: Response = client.get("/api", headers=headers)
        assert response.status_code == 200


def test_protected_route_unauthenticated(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        response: Response = client.get("/api")
        assert response.status_code == 401


def test_protected_route_no_auth_configured(client: TestClient) -> None:
    with patch.object(settings, "SECRET_KEY", None):
        response: Response = client.get("/api")
        assert response.status_code == 200


def test_streaming_endpoint_is_public(client: TestClient) -> None:
    """Test that streaming endpoints are public and don't require authentication."""
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        # Test streaming endpoint without authentication
        response: Response = client.get("/api/v1/tracks/123/stream")
        # Should not return 401 (would return 404 since track doesn't exist, but that's fine)
        assert response.status_code != 401


def test_options_request_allowed_for_cors(client: TestClient) -> None:
    """Test that OPTIONS requests are allowed to pass through for CORS preflight."""
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        # Test OPTIONS request to a protected endpoint without authentication
        response: Response = client.options("/api/v1/tracks")
        # Should not return 401 (CORS preflight should be allowed)
        assert response.status_code != 401


def test_cover_endpoint_is_public(client: TestClient) -> None:
    """Test that cover endpoints are public and don't require authentication."""
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        # Test cover endpoint without authentication
        response: Response = client.get("/api/v1/tracks/123/covers/small.webp")
        # Should not return 401 (would return 404 since track doesn't exist, but that's fine)
        assert response.status_code != 401


def test_player_state_endpoint_is_public(client: TestClient) -> None:
    """Test that player state endpoints are public and don't require authentication."""
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        # Test GET player state endpoint without authentication
        response: Response = client.get("/api/v1/player/state")
        # Should not return 401 (might return 200 with default state)
        assert response.status_code != 401

        # Test POST player state endpoint without authentication
        player_state = {
            "current_track_id": 1,
            "progress_seconds": 30.0,
            "volume_level": 0.8,
            "is_muted": False,
            "is_shuffle": False,
            "is_repeat": False,
        }
        response: Response = client.post("/api/v1/player/state", json=player_state)
        # Should not return 401 (beacons need to work without auth headers)
        assert response.status_code != 401
