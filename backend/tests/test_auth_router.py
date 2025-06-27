import pytest
from fastapi.testclient import TestClient
from unittest.mock import patch

from src.mus.config import settings


@pytest.fixture
def client(app):
    with TestClient(app) as test_client:
        yield test_client


def test_get_qr_code_url_with_secret_key(client: TestClient):
    with patch.object(settings, "SECRET_KEY", "test-secret-key"):
        response = client.get("/api/v1/auth/qr-code-url")

        assert response.status_code == 200
        data = response.json()
        assert data["url"] == "/login?token=test-secret-key"


def test_get_qr_code_url_without_secret_key(client: TestClient):
    with patch.object(settings, "SECRET_KEY", None):
        response = client.get("/api/v1/auth/qr-code-url")

        assert response.status_code == 200
        data = response.json()
        assert data["url"] == ""


def test_get_qr_code_url_with_empty_secret_key(client: TestClient):
    with patch.object(settings, "SECRET_KEY", ""):
        response = client.get("/api/v1/auth/qr-code-url")

        assert response.status_code == 200
        data = response.json()
        assert data["url"] == ""
