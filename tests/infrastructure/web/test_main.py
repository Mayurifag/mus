import os
import tempfile
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from mus.infrastructure.web.main import app


@pytest.fixture
def client():
    # Create temporary directories for testing
    with tempfile.TemporaryDirectory() as temp_dir:
        music_dir = Path(temp_dir) / "music"
        music_dir.mkdir()
        db_dir = Path(temp_dir) / "data"
        db_dir.mkdir()
        db_path = db_dir / "test.db"

        # Set environment variables
        os.environ["MUSIC_DIR"] = str(music_dir)
        os.environ["DATABASE_PATH"] = str(db_path)

        yield TestClient(app)


def test_root_route(client):
    response = client.get("/")
    assert response.status_code == 200
    assert "MUS - Music Player" in response.text


def test_get_tracks_empty(client):
    response = client.get("/tracks")
    assert response.status_code == 200
    assert "No tracks found" in response.text


def test_scan_tracks(client):
    response = client.post("/scan")
    assert response.status_code == 200
    assert "Scan completed" in response.text


def test_stream_audio_success(client):
    # Create a test audio file
    music_dir = Path(os.environ["MUSIC_DIR"])
    test_file = music_dir / "test.mp3"
    test_file.write_bytes(b"fake audio data")

    response = client.get("/stream/test.mp3")
    assert response.status_code == 200
    assert response.headers["content-type"] == "audio/mpeg"
    assert response.content == b"fake audio data"


def test_stream_audio_not_found(client):
    response = client.get("/stream/nonexistent.mp3")
    assert response.status_code == 404
    assert "File not found" in response.text


def test_stream_audio_forbidden(client):
    # Try to access a file outside the music directory using a different traversal pattern
    response = client.get("/stream/..%2F..%2F..%2Fetc%2Fpasswd")
    assert response.status_code == 403
    assert "Access denied" in response.text
