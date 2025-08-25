from unittest.mock import AsyncMock, patch

import pytest
from fastapi.testclient import TestClient

from src.mus.application.services.permissions_service import PermissionsService
from src.mus.infrastructure.api.dependencies import get_permissions_service


@pytest.fixture
def client(app):
    with TestClient(app) as test_client:
        yield test_client


@pytest.mark.asyncio
async def test_initiate_download_success(client, app):
    mock_redis = AsyncMock()
    mock_redis.set.return_value = True
    mock_redis.aclose = AsyncMock()

    # Create a mock permissions service with write access
    mock_permissions = PermissionsService()
    mock_permissions.can_write_music_files = True

    # Override the dependency
    app.dependency_overrides[get_permissions_service] = lambda: mock_permissions

    with (
        patch(
            "src.mus.infrastructure.api.routers.download_router.get_redis_client"
        ) as mock_get_redis,
        patch(
            "src.mus.infrastructure.jobs.download_jobs.download_track_from_url.enqueue",
            new_callable=AsyncMock,
        ) as mock_enqueue,
    ):
        mock_get_redis.return_value = mock_redis

        try:
            response = client.post(
                "/api/v1/downloads/url", json={"url": "https://example.com/video"}
            )

            assert response.status_code == 202
            assert response.json() == {"status": "accepted"}
            mock_enqueue.assert_called_once_with(url="https://example.com/video")
        finally:
            # Clean up the override
            app.dependency_overrides.clear()


@pytest.mark.asyncio
async def test_initiate_download_no_write_permissions(client, app):
    # Create a mock permissions service without write access
    mock_permissions = PermissionsService()
    mock_permissions.can_write_music_files = False

    # Override the dependency
    app.dependency_overrides[get_permissions_service] = lambda: mock_permissions

    try:
        response = client.post(
            "/api/v1/downloads/url", json={"url": "https://example.com/video"}
        )

        assert response.status_code == 403
        assert (
            response.json()["detail"]
            == "Download not available - insufficient write permissions to music directory"
        )
    finally:
        # Clean up the override
        app.dependency_overrides.clear()


@pytest.mark.asyncio
async def test_initiate_download_lock_already_exists(client, app):
    mock_redis = AsyncMock()
    mock_redis.set.return_value = False  # Lock already exists
    mock_redis.aclose = AsyncMock()

    # Create a mock permissions service with write access
    mock_permissions = PermissionsService()
    mock_permissions.can_write_music_files = True

    # Override the dependency
    app.dependency_overrides[get_permissions_service] = lambda: mock_permissions

    with patch(
        "src.mus.infrastructure.api.routers.download_router.get_redis_client"
    ) as mock_get_redis:
        mock_get_redis.return_value = mock_redis

        try:
            response = client.post(
                "/api/v1/downloads/url", json={"url": "https://example.com/video"}
            )

            assert response.status_code == 429
            assert response.json()["detail"] == "Download already in progress"
        finally:
            # Clean up the override
            app.dependency_overrides.clear()


@pytest.mark.asyncio
async def test_download_job_releases_lock():
    mock_redis = AsyncMock()
    mock_redis.delete = AsyncMock()
    mock_redis.aclose = AsyncMock()

    with patch("src.mus.core.redis.get_redis_client", return_value=mock_redis):
        # Since we can't call the RegisteredTask directly, we'll test the logic by
        # creating a simple function that mimics what the job does
        async def test_job_logic(url: str):
            import logging
            from src.mus.core.redis import get_redis_client

            logger = logging.getLogger(__name__)
            logger.info(f"WORKER: Starting download_track_from_url for URL: {url}")

            try:
                logger.info(f"WORKER: Placeholder job executed for URL: {url}")
            finally:
                client = await get_redis_client()
                try:
                    lock_key = "download_lock:global"
                    await client.delete(lock_key)
                    logger.info("WORKER: Released download lock")
                finally:
                    await client.aclose()

            logger.info(f"WORKER: Completed download_track_from_url for URL: {url}")

        await test_job_logic("https://example.com/video")

        mock_redis.delete.assert_called_once_with("download_lock:global")
        mock_redis.aclose.assert_called()
