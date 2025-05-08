import os
import tempfile
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

from mus.dependencies import get_track_repository
from mus.domain.track import Track
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


async def test_stream_audio_by_id_success(client):
    # Create a test audio file
    music_dir = Path(os.environ["MUSIC_DIR"])
    test_file = music_dir / "test.mp3"
    test_file.write_bytes(b"fake audio data")

    # Add a track to the repository
    repository = get_track_repository()
    track = Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,
        file_path=test_file,
        added_at=1234567890,
    )
    await repository.add(track)
    tracks = await repository.get_all()
    track_id = tracks[0].id

    response = client.get(f"/stream/{track_id}")
    assert response.status_code == 200
    assert response.headers["content-type"] == "audio/mpeg"
    assert response.content == b"fake audio data"


async def test_stream_audio_by_id_track_not_found(client):
    response = client.get("/stream/999")
    assert response.status_code == 404


async def test_stream_audio_by_id_file_not_found(client):
    # Add a track with a non-existent file
    repository = get_track_repository()
    track = Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,
        file_path=Path(os.environ["MUSIC_DIR"]) / "nonexistent.mp3",
        added_at=1234567890,
    )
    await repository.add(track)
    tracks = await repository.get_all()
    track_id = tracks[0].id

    response = client.get(f"/stream/{track_id}")
    assert response.status_code == 404
