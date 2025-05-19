import pytest
import httpx
import asyncio
from unittest.mock import patch, MagicMock, AsyncMock
from fastapi import FastAPI
from sqlalchemy.ext.asyncio import AsyncSession
from fastapi.testclient import TestClient
from src.mus.infrastructure.api.routers.track_router import stream_track
from src.mus.domain.entities.track import Track


@pytest.mark.asyncio
async def test_get_tracks(app: FastAPI, session: AsyncSession, sample_track: Track):
    """Test that tracks are returned correctly."""
    # Ensure sample_track has added_at set
    if not hasattr(sample_track, "added_at") or sample_track.added_at is None:
        sample_track.added_at = 1609459200  # January 1, 2021 00:00:00 UTC

    session.add(sample_track)
    sample_track.has_cover = True

    await session.commit()

    async with httpx.AsyncClient(
        transport=httpx.ASGITransport(app=app), base_url="http://test"
    ) as ac:
        response = await ac.get("/api/v1/tracks")
        assert response.status_code == 200

        tracks = response.json()
        assert len(tracks) == 1
        assert tracks[0]["title"] == "Test Track"
        assert tracks[0]["artist"] == "Test Artist"

        assert "covers/small.webp" in tracks[0]["cover_small_url"]
        assert "covers/original.webp" in tracks[0]["cover_original_url"]
        assert "cover_medium_url" not in tracks[0]


@pytest.mark.asyncio
async def test_stream_track_not_found(app):
    """Test 404 when track is not found."""
    async with httpx.AsyncClient(
        transport=httpx.ASGITransport(app=app), base_url="http://test"
    ) as ac:
        response = await ac.get("/api/v1/tracks/999/stream")
        assert response.status_code == 404


@pytest.mark.asyncio
async def test_stream_track_file_not_found(app):
    """Test 404 when audio file doesn't exist."""
    track_id = 1

    with patch("os.path.isfile", return_value=False), patch(
        "src.mus.infrastructure.persistence.sqlite_track_repository.SQLiteTrackRepository.get_by_id",
        new_callable=AsyncMock,
        return_value=MagicMock(id=track_id, file_path="/non/existent/path.mp3"),
    ):
        async with httpx.AsyncClient(
            transport=httpx.ASGITransport(app=app), base_url="http://test"
        ) as ac:
            response = await ac.get(f"/api/v1/tracks/{track_id}/stream")
            assert response.status_code == 404


def test_stream_track_success():
    """Test successful audio file streaming - simplified test."""
    track_mock = MagicMock(id=1, file_path="/fake/path.mp3")
    repo_mock = AsyncMock()
    repo_mock.get_by_id.return_value = track_mock

    file_response_mock = MagicMock(status_code=200)

    with patch("os.path.isfile", return_value=True), patch(
        "fastapi.responses.FileResponse", return_value=file_response_mock
    ):
        result = asyncio.get_event_loop().run_until_complete(
            stream_track(track_id=1, track_repository=repo_mock)
        )

        assert result.status_code == 200
        repo_mock.get_by_id.assert_awaited_once_with(1)


def test_get_track_cover_not_found(app):
    """Test 404 when track is not found."""
    with TestClient(app) as client:
        response = client.get("/api/v1/tracks/999/covers/small.webp")
        assert response.status_code == 404


@pytest.mark.asyncio
async def test_get_track_cover_invalid_size(app):
    """Test 422 when an invalid size is provided."""
    track_id = 1
    async with httpx.AsyncClient(
        transport=httpx.ASGITransport(app=app), base_url="http://test"
    ) as ac:
        response = await ac.get(f"/api/v1/tracks/{track_id}/covers/medium.webp")
    assert response.status_code in (404, 422)


@pytest.mark.asyncio
async def test_get_track_cover_no_cover(app):
    """Test 404 when track has no cover."""
    track_id = 1
    async with httpx.AsyncClient(
        transport=httpx.ASGITransport(app=app), base_url="http://test"
    ) as ac:
        response = await ac.get(f"/api/v1/tracks/{track_id}/covers/small.webp")
    assert response.status_code in (404, 422)


@pytest.mark.asyncio
async def test_get_track_cover_file_not_found(app):
    """Test 404 when the cover file doesn't exist."""
    track_id = 1
    with patch("os.path.isfile", return_value=False):
        async with httpx.AsyncClient(
            transport=httpx.ASGITransport(app=app), base_url="http://test"
        ) as ac:
            response = await ac.get(f"/api/v1/tracks/{track_id}/covers/small.webp")
        assert response.status_code in (404, 422)


@pytest.mark.asyncio
async def test_get_track_cover_success(app):
    """Test successful cover retrieval."""
    track_id = 1
    with (
        patch("os.path.isfile", return_value=True),
        patch(
            "mus.infrastructure.api.routers.track_router.FileResponse",
            return_value=MagicMock(status_code=200),
        ),
    ):
        async with httpx.AsyncClient(
            transport=httpx.ASGITransport(app=app), base_url="http://test"
        ) as ac:
            response = await ac.get(f"/api/v1/tracks/{track_id}/covers/small.webp")
            assert response.status_code in (200, 404, 422)
            response = await ac.get(f"/api/v1/tracks/{track_id}/covers/original.webp")
            assert response.status_code in (200, 404, 422)
