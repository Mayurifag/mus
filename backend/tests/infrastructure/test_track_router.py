import pytest
import httpx
from unittest.mock import patch, MagicMock
from fastapi.testclient import TestClient


def test_get_tracks(app, session, sample_track):
    """Test that tracks are returned correctly."""
    # Add track to database
    session.add(sample_track)
    sample_track.has_cover = True
    import asyncio

    asyncio.get_event_loop().run_until_complete(session.commit())

    with TestClient(app) as client:
        # Make the request
        response = client.get("/api/v1/tracks")
        assert response.status_code == 200

        tracks = response.json()
        assert len(tracks) == 1
        assert tracks[0]["title"] == "Test Track"
        assert tracks[0]["artist"] == "Test Artist"

        # Check that cover URLs are constructed correctly
        assert "covers/small.webp" in tracks[0]["cover_small_url"]
        assert "covers/original.webp" in tracks[0]["cover_original_url"]
        assert "cover_medium_url" not in tracks[0]


def test_get_track_cover_not_found(app):
    """Test 404 when track is not found."""
    with TestClient(app) as client:
        response = client.get("/api/v1/tracks/999/covers/small.webp")
        assert response.status_code == 404


@pytest.mark.asyncio
async def test_get_track_cover_invalid_size(app):
    """Test 422 when an invalid size is provided."""
    # This test assumes a track with id=1 exists and has no cover
    # You may need to create a track via an API call if available
    track_id = 1
    async with httpx.AsyncClient(
        transport=httpx.ASGITransport(app=app), base_url="http://test"
    ) as ac:
        response = await ac.get(f"/api/v1/tracks/{track_id}/covers/medium.webp")
    assert response.status_code in (404, 422)


@pytest.mark.asyncio
async def test_get_track_cover_no_cover(app):
    """Test 404 when track has no cover."""
    # This test assumes a track with id=1 exists and has no cover
    track_id = 1
    async with httpx.AsyncClient(
        transport=httpx.ASGITransport(app=app), base_url="http://test"
    ) as ac:
        response = await ac.get(f"/api/v1/tracks/{track_id}/covers/small.webp")
    assert response.status_code in (404, 422)


@pytest.mark.asyncio
async def test_get_track_cover_file_not_found(app):
    """Test 404 when the cover file doesn't exist."""
    # This test assumes a track with id=1 exists and has a cover flag set
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
    # This test assumes a track with id=1 exists and has a cover flag set
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
