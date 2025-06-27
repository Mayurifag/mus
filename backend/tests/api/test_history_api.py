import pytest
from unittest.mock import patch, AsyncMock
from fastapi.testclient import TestClient

from src.mus.domain.entities.track import Track
from src.mus.domain.entities.track_history import TrackHistory
from src.mus.main import app

warning_filter = "ignore::sqlalchemy.exc.SAWarning"


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_get_track_history():
    history_entry = TrackHistory(
        id=1,
        track_id=1,
        title="Old Title",
        artist="Old Artist",
        duration=120,
        changed_at=1609459100,
    )

    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_history_repository.SQLiteTrackHistoryRepository.get_by_track_id",
        return_value=[history_entry],
    ):
        client = TestClient(app)
        response = client.get("/api/v1/tracks/1/history")

        assert response.status_code == 200
        data = response.json()
        assert len(data) == 1
        assert data[0]["track_id"] == 1
        assert data[0]["title"] == "Old Title"
        assert data[0]["artist"] == "Old Artist"
        assert data[0]["duration"] == 120
        assert data[0]["changed_at"] == 1609459100


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_get_track_history_empty():
    with patch(
        "src.mus.infrastructure.persistence.sqlite_track_history_repository.SQLiteTrackHistoryRepository.get_by_track_id",
        return_value=[],
    ):
        client = TestClient(app)
        response = client.get("/api/v1/tracks/1/history")

        assert response.status_code == 200
        data = response.json()
        assert len(data) == 0


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_rollback_track_history():
    history_entry = TrackHistory(
        id=1,
        track_id=1,
        title="Old Title",
        artist="Old Artist",
        duration=120,
        changed_at=1609459100,
    )

    track = Track(
        id=1,
        title="Current Title",
        artist="Current Artist",
        duration=180,
        file_path="/path/to/test.mp3",
        added_at=1609459200,
    )

    with (
        patch(
            "src.mus.infrastructure.persistence.sqlite_track_history_repository.SQLiteTrackHistoryRepository.get_by_id",
            return_value=history_entry,
        ),
        patch(
            "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
            return_value=track,
        ),
        patch(
            "src.mus.infrastructure.api.sse_handler.notify_sse_from_worker",
            return_value=AsyncMock(),
        ),
        patch("sqlmodel.ext.asyncio.session.AsyncSession.add"),
        patch(
            "sqlmodel.ext.asyncio.session.AsyncSession.commit", return_value=AsyncMock()
        ),
    ):
        client = TestClient(app)
        response = client.post("/api/v1/tracks/history/1/rollback")

        assert response.status_code == 200
        data = response.json()
        assert data["message"] == "Track rolled back successfully"


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_rollback_track_history_not_found(app):
    client = TestClient(app)
    response = client.post("/api/v1/tracks/history/999/rollback")

    assert response.status_code == 404
    data = response.json()
    assert data["detail"] == "History entry not found"
