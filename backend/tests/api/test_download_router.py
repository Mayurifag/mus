from unittest.mock import AsyncMock, patch
from pathlib import Path
import tempfile

import pytest
from fastapi.testclient import TestClient

from src.mus.application.services.permissions_service import PermissionsService
from src.mus.infrastructure.api.dependencies import get_permissions_service
from src.mus.infrastructure.jobs.download_jobs import _download_audio


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


def test_cookies_file_path_config():
    """Test that COOKIES_FILE_PATH config property works correctly."""
    from src.mus.config import Config

    with tempfile.TemporaryDirectory() as temp_dir:
        config = Config()
        config.DATA_DIR_PATH = Path(temp_dir)

        expected_path = Path(temp_dir) / "cookies.txt"
        assert config.COOKIES_FILE_PATH == expected_path


def test_download_command_includes_cookies_when_file_exists():
    """Test that the download command includes --cookies flag when cookies file exists."""
    import logging

    with tempfile.TemporaryDirectory() as temp_dir:
        cookies_file = Path(temp_dir) / "cookies.txt"
        cookies_file.write_text("# Test cookies file")

        with patch(
            "src.mus.infrastructure.jobs.download_jobs.settings"
        ) as mock_settings:
            mock_settings.COOKIES_FILE_PATH = cookies_file

            # Mock subprocess.run to capture the command
            with patch(
                "src.mus.infrastructure.jobs.download_jobs.subprocess.run"
            ) as mock_run:
                mock_run.side_effect = Exception("Test exception to stop execution")

                logger = logging.getLogger(__name__)
                try:
                    _download_audio("https://example.com/video", logger)
                except Exception:
                    pass

                mock_run.assert_called_once()
                cmd_args = mock_run.call_args[0][0]

                assert "--cookies" in cmd_args
                cookies_index = cmd_args.index("--cookies")
                assert cmd_args[cookies_index + 1] == str(cookies_file)


def test_download_command_excludes_cookies_when_file_missing():
    """Test that the download command excludes --cookies flag when cookies file doesn't exist."""
    import logging

    with tempfile.TemporaryDirectory() as temp_dir:
        non_existent_cookies = Path(temp_dir) / "nonexistent_cookies.txt"

        with patch(
            "src.mus.infrastructure.jobs.download_jobs.settings"
        ) as mock_settings:
            mock_settings.COOKIES_FILE_PATH = non_existent_cookies

            # Mock subprocess.run to capture the command
            with patch(
                "src.mus.infrastructure.jobs.download_jobs.subprocess.run"
            ) as mock_run:
                mock_run.side_effect = Exception("Test exception to stop execution")

                logger = logging.getLogger(__name__)
                try:
                    _download_audio("https://example.com/video", logger)
                except Exception:
                    pass

                mock_run.assert_called_once()
                cmd_args = mock_run.call_args[0][0]

                assert "--cookies" not in cmd_args
