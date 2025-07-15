from unittest.mock import AsyncMock, MagicMock, Mock, patch

import pytest
import pytest_asyncio
from fastapi.testclient import TestClient
from sqlmodel import select, text

from src.mus.config import settings
from src.mus.domain.entities.track import Track
from src.mus.infrastructure.api.routers.track_router import (
    CoverSize,
    _generate_etag,
    get_track_cover,
    stream_track,
)


@pytest.fixture
def client(app):
    with TestClient(app) as test_client:
        yield test_client


@pytest_asyncio.fixture
async def clear_tracks(track_repository):
    query = text("DELETE FROM track")
    await track_repository.session.exec(query)
    await track_repository.session.commit()

    results = await track_repository.session.exec(select(Track))
    tracks = results.all()
    assert len(tracks) == 0


@pytest_asyncio.fixture
async def sample_tracks(track_repository):
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

    for track in tracks:
        await track_repository.add(track)

    results = await track_repository.session.exec(select(Track))
    saved_tracks = results.all()
    assert len(saved_tracks) == 2

    return tracks


@pytest.mark.asyncio
async def test_get_tracks(client, sample_tracks):
    # Create mock Row objects that match what the repository now returns
    from sqlalchemy.engine import Row
    from unittest.mock import MagicMock

    mock_rows = []
    for track in sample_tracks:
        mock_row = MagicMock(spec=Row)
        mock_row.id = track.id
        mock_row.title = track.title
        mock_row.artist = track.artist
        mock_row.duration = track.duration
        mock_row.has_cover = track.has_cover
        mock_row.file_path = track.file_path
        mock_row.updated_at = track.updated_at
        mock_rows.append(mock_row)

    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_all",
        return_value=mock_rows,
    ):
        response = client.get("/api/v1/tracks")

        assert response.status_code == 200
        data = response.json()

        assert len(data) == 2
        assert data[0]["id"] == 1
        assert data[0]["title"] == "Test Track 1"
        assert data[1]["id"] == 2
        assert data[1]["title"] == "Test Track 2"
        # Ensure file_path is in the response but added_at is not
        assert "file_path" in data[0]
        assert "added_at" not in data[0]
        assert "file_path" in data[1]
        assert "added_at" not in data[1]


@pytest.mark.asyncio
async def test_get_tracks_empty(client):
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
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=None,
    ):
        response = client.get("/api/v1/tracks/999/stream")

        assert response.status_code == 404
        assert response.json()["detail"] == "Track with ID 999 not found"


@pytest.mark.asyncio
async def test_stream_track_file_not_found(client, sample_tracks):
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=sample_tracks[0],
    ):
        with patch("os.path.isfile", return_value=False):
            response = client.get("/api/v1/tracks/1/stream")

            assert response.status_code == 404
            assert response.json()["detail"] == "Audio file for track 1 not found"


@pytest.mark.asyncio
async def test_stream_track_success(client, sample_tracks):
    repo_mock = AsyncMock()
    repo_mock.get_by_id.return_value = sample_tracks[0]

    file_response_mock = MagicMock(status_code=200)

    # Mock request object
    mock_request = MagicMock()
    mock_request.headers = {}

    with (
        patch("os.path.isfile", return_value=True),
        patch("fastapi.responses.FileResponse", return_value=file_response_mock),
        patch("asyncio.to_thread") as mock_to_thread,
    ):
        # Mock os.stat result
        mock_stat = MagicMock()
        mock_stat.st_size = 1024
        mock_stat.st_mtime = 1609459200.0
        mock_to_thread.return_value = mock_stat

        result = await stream_track(
            request=mock_request, track_id=1, track_repository=repo_mock
        )

        assert result.status_code == 200
        repo_mock.get_by_id.assert_called_once_with(1)


@pytest.mark.asyncio
async def test_get_track_cover_no_cover(client, sample_tracks):
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=sample_tracks[1],
    ):
        response = client.get("/api/v1/tracks/2/covers/small.webp")

        assert response.status_code == 404
        assert response.json()["detail"] == "This track has no cover image"


@pytest.mark.asyncio
async def test_get_track_cover_file_not_found(client, sample_tracks):
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        return_value=sample_tracks[0],
    ):
        with patch("os.path.isfile", return_value=False):
            response = client.get("/api/v1/tracks/1/covers/small.webp")

            assert response.status_code == 404
            assert "Cover image for track 1 with size" in response.json()["detail"]
            assert "not found" in response.json()["detail"]


@pytest.mark.asyncio
async def test_get_track_cover_success_small(client, sample_tracks):
    repo_mock = AsyncMock()
    repo_mock.get_by_id.return_value = sample_tracks[0]

    file_response_mock = MagicMock(status_code=200)
    mock_stat = MagicMock()
    mock_stat.st_size = 1024
    mock_stat.st_mtime = 1609459200.0

    with (
        patch("os.path.isfile", return_value=True),
        patch("os.stat", return_value=mock_stat),
        patch("fastapi.responses.FileResponse", return_value=file_response_mock),
    ):
        mock_request = Mock()
        mock_request.headers = {}

        result = await get_track_cover(
            request=mock_request,
            track_id=1,
            size=CoverSize.SMALL,
            track_repository=repo_mock,
        )

        assert result.status_code == 200
        repo_mock.get_by_id.assert_called_once_with(1)


@pytest.mark.asyncio
async def test_get_track_cover_success_original(client, sample_tracks):
    repo_mock = AsyncMock()
    repo_mock.get_by_id.return_value = sample_tracks[0]

    file_response_mock = MagicMock(status_code=200)
    mock_stat = MagicMock()
    mock_stat.st_size = 1024
    mock_stat.st_mtime = 1609459200.0

    with (
        patch("os.path.isfile", return_value=True),
        patch("os.stat", return_value=mock_stat),
        patch("fastapi.responses.FileResponse", return_value=file_response_mock),
    ):
        mock_request = Mock()
        mock_request.headers = {}

        result = await get_track_cover(
            request=mock_request,
            track_id=1,
            size=CoverSize.ORIGINAL,
            track_repository=repo_mock,
        )

        assert result.status_code == 200
        repo_mock.get_by_id.assert_called_once_with(1)


@pytest.mark.asyncio
async def test_get_track_cover_etag_and_cache_headers(client, sample_tracks):
    repo_mock = AsyncMock()
    repo_mock.get_by_id.return_value = sample_tracks[0]

    mock_stat = MagicMock()
    mock_stat.st_size = 1024
    mock_stat.st_mtime = 1609459200.0

    with (
        patch("os.path.isfile", return_value=True),
        patch("os.stat", return_value=mock_stat),
    ):
        mock_request = Mock()
        mock_request.headers = {}

        result = await get_track_cover(
            request=mock_request,
            track_id=1,
            size=CoverSize.SMALL,
            track_repository=repo_mock,
        )

        assert hasattr(result, "headers")
        assert "ETag" in result.headers
        assert "Cache-Control" in result.headers
        assert result.headers["Cache-Control"] == "public, max-age=31536000, immutable"


@pytest.mark.asyncio
async def test_get_track_cover_304_not_modified(client, sample_tracks):
    repo_mock = AsyncMock()
    repo_mock.get_by_id.return_value = sample_tracks[0]

    mock_stat = MagicMock()
    mock_stat.st_size = 1024
    mock_stat.st_mtime = 1609459200.0

    with (
        patch("os.path.isfile", return_value=True),
        patch("os.stat", return_value=mock_stat),
    ):
        expected_file_path = str(settings.COVERS_DIR_PATH / "1_small.webp")
        expected_etag = _generate_etag(expected_file_path, 1024, 1609459200.0)

        mock_request = Mock()
        mock_request.headers = {"if-none-match": f'"{expected_etag}"'}

        result = await get_track_cover(
            request=mock_request,
            track_id=1,
            size=CoverSize.SMALL,
            track_repository=repo_mock,
        )

        assert result.status_code == 304
