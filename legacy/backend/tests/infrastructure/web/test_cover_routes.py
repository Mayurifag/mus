import os
from pathlib import Path
from unittest.mock import patch

import pytest
import structlog
from fastapi.testclient import TestClient

from mus.infrastructure.web.main import app

log = structlog.get_logger()

COVERS_DIR = os.environ.get("COVERS_DIR", "/app/data/covers")
PLACEHOLDER_SVG_PATH = "src/mus/infrastructure/web/static/images/placeholder.svg"


@pytest.fixture
def test_client():
    return TestClient(app)


@pytest.fixture
def mock_covers_dir(tmp_path):
    original_covers_dir = os.environ.get("COVERS_DIR")
    os.environ["COVERS_DIR"] = str(tmp_path)
    log.info("Set COVERS_DIR", covers_dir=str(tmp_path))
    yield tmp_path
    if original_covers_dir:
        os.environ["COVERS_DIR"] = original_covers_dir
    else:
        del os.environ["COVERS_DIR"]


@pytest.fixture
def mock_placeholder_svg():
    with open(PLACEHOLDER_SVG_PATH, "rb") as f:
        return f.read()


class MockResponse:
    def __init__(self, content, media_type, status_code=200):
        self.content = content
        self.headers = {"content-type": media_type}
        self.status_code = status_code


def test_get_cover_small(test_client, mock_covers_dir):
    # Create a test cover file
    cover_path = Path(os.environ["COVERS_DIR"]) / "1_small.webp"
    cover_path.write_bytes(b"fake_webp_data")
    log.info("Created test cover file", path=str(cover_path))

    def mock_file_response(path, media_type=None, **kwargs):
        path = Path(path)
        log.info("Mock FileResponse called", path=str(path), media_type=media_type)

        if path.suffix == ".webp":
            covers_dir = Path(os.environ["COVERS_DIR"])
            cover_path = covers_dir / path.name
            log.info(
                "Checking cover path",
                cover_path=str(cover_path),
                exists=cover_path.exists(),
            )
            if cover_path.exists():
                content = cover_path.read_bytes()
                return MockResponse(content, "image/webp")

        # For non-existent WebP files or placeholder SVG, return placeholder
        with open(PLACEHOLDER_SVG_PATH, "rb") as f:
            return MockResponse(f.read(), "image/svg+xml")

    with patch("fastapi.responses.FileResponse", side_effect=mock_file_response):
        response = test_client.get("/covers/small/1.webp")
        log.info(
            "Got response",
            status_code=response.status_code,
            content_type=response.headers.get("content-type"),
        )
        assert response.status_code == 200


def test_get_cover_medium(test_client, mock_covers_dir):
    # Create a test cover file
    cover_path = Path(os.environ["COVERS_DIR"]) / "1_medium.webp"
    cover_path.write_bytes(b"fake_webp_data")
    log.info("Created test cover file", path=str(cover_path))

    def mock_file_response(path, media_type=None, **kwargs):
        path = Path(path)
        log.info("Mock FileResponse called", path=str(path), media_type=media_type)

        if path.suffix == ".webp":
            covers_dir = Path(os.environ["COVERS_DIR"])
            cover_path = covers_dir / path.name
            log.info(
                "Checking cover path",
                cover_path=str(cover_path),
                exists=cover_path.exists(),
            )
            if cover_path.exists():
                content = cover_path.read_bytes()
                return MockResponse(content, "image/webp")

        # For non-existent WebP files or placeholder SVG, return placeholder
        with open(PLACEHOLDER_SVG_PATH, "rb") as f:
            return MockResponse(f.read(), "image/svg+xml")

    with patch("fastapi.responses.FileResponse", side_effect=mock_file_response):
        response = test_client.get("/covers/medium/1.webp")
        log.info(
            "Got response",
            status_code=response.status_code,
            content_type=response.headers.get("content-type"),
        )
        assert response.status_code == 200


def test_get_cover_invalid_size(test_client):
    response = test_client.get("/covers/invalid/1.webp")
    assert response.status_code == 400
    assert response.json()["detail"] == "Invalid size"


def test_get_cover_not_found(test_client, mock_covers_dir, mock_placeholder_svg):
    def mock_file_response(path, media_type=None, **kwargs):
        return MockResponse(mock_placeholder_svg, "image/svg+xml")

    with patch("fastapi.responses.FileResponse", side_effect=mock_file_response):
        response = test_client.get("/covers/small/999.webp")
        assert response.status_code == 200
        assert response.headers["content-type"] == "image/svg+xml"
        assert response.content == mock_placeholder_svg
