import pytest
import pytest_asyncio
from fastapi.testclient import TestClient
from unittest.mock import patch
from sqlmodel import select, text

from src.mus.domain.entities.track import Track
from src.mus.main import app


@pytest.fixture
def client():
    with TestClient(app) as test_client:
        yield test_client


@pytest_asyncio.fixture
async def clear_tracks(track_repository):
    # Query for all tracks and delete them to ensure clean state
    query = text("DELETE FROM track")
    await track_repository.session.exec(query)
    await track_repository.session.commit()

    # Verify no tracks exist
    results = await track_repository.session.exec(select(Track))
    tracks = results.all()
    assert len(tracks) == 0


@pytest_asyncio.fixture
async def sample_tracks(track_repository, clear_tracks):
    # Create test tracks after clearing any existing ones
    tracks = [
        Track(
            id=1,
            title="Test Track 1",
            artist="Test Artist 1",
            duration=180,
            file_path="/path/to/test1.mp3",
            has_cover=True,
            added_at=1746920951,
        ),
        Track(
            id=2,
            title="Test Track 2",
            artist="Test Artist 2",
            duration=240,
            file_path="/path/to/test2.mp3",
            has_cover=False,
            added_at=1747118899,
        ),
    ]

    # Add tracks to repository
    for track in tracks:
        await track_repository.add(track)

    # Verify tracks were added
    results = await track_repository.session.exec(select(Track))
    saved_tracks = results.all()
    assert len(saved_tracks) == 2

    return tracks


@pytest.mark.asyncio
async def test_get_tracks(client, sample_tracks):
    # Patch get_all to return our sample tracks directly
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_all",
        return_value=sample_tracks,
    ):
        response = client.get("/api/v1/tracks")

        assert response.status_code == 200
        data = response.json()

        assert len(data) == 2
        assert data[0]["id"] == 1
        assert data[0]["title"] == "Test Track 1"
        assert data[1]["id"] == 2
        assert data[1]["title"] == "Test Track 2"


@pytest.mark.asyncio
async def test_get_tracks_empty(client, clear_tracks):
    # Patch get_all to return an empty list
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_all",
        return_value=[],
    ):
        response = client.get("/api/v1/tracks")

        assert response.status_code == 200
        data = response.json()
        assert len(data) == 0


@pytest.mark.asyncio
async def test_stream_track_not_found(client, sample_tracks):
    # Patch to return None for non-existent track
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=None,
    ):
        response = client.get("/api/v1/tracks/999/stream")

        assert response.status_code == 404
        assert response.json()["detail"] == "Track with ID 999 not found"


@pytest.mark.asyncio
async def test_stream_track_file_not_found(client, sample_tracks):
    # Patch repository to return track but file doesn't exist
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=sample_tracks[0],
    ):
        with patch("os.path.isfile", return_value=False):
            response = client.get("/api/v1/tracks/1/stream")

            assert response.status_code == 404
            assert response.json()["detail"] == "Audio file for track 1 not found"


@pytest.mark.asyncio
async def test_get_track_cover_no_cover(client, sample_tracks):
    # Patch to return a track that has no cover
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=sample_tracks[1],
    ):  # Track with has_cover=False
        response = client.get("/api/v1/tracks/2/covers/small.webp")

        assert response.status_code == 404
        assert response.json()["detail"] == "This track has no cover image"


@pytest.mark.asyncio
async def test_get_track_cover_file_not_found(client, sample_tracks):
    # Patch to return a track with cover but file doesn't exist
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=sample_tracks[0],
    ):  # Track with has_cover=True
        with patch("os.path.isfile", return_value=False):
            response = client.get("/api/v1/tracks/1/covers/small.webp")

            assert response.status_code == 404
            # The actual error message includes the enum value, so we need to match that
            assert "Cover image for track 1 with size" in response.json()["detail"]
            assert "not found" in response.json()["detail"]
