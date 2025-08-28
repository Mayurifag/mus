"""
Tests for YtDlpService.
"""

import asyncio
import subprocess
import sys
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from src.mus.infrastructure.services.ytdlp_service import YtDlpService


@pytest.fixture
def ytdlp_service():
    """Create a YtDlpService instance for testing."""
    return YtDlpService()


@pytest.mark.asyncio
async def test_ensure_ytdlp_proxy_available_success(ytdlp_service):
    """Test successful yt-dlp-proxy availability check."""
    mock_result = MagicMock()
    mock_result.returncode = 0
    
    with patch('asyncio.to_thread', return_value=mock_result):
        result = await ytdlp_service.ensure_ytdlp_proxy_available()
        assert result is True


@pytest.mark.asyncio
async def test_ensure_ytdlp_proxy_available_failure(ytdlp_service):
    """Test failed yt-dlp-proxy availability check."""
    mock_result = MagicMock()
    mock_result.returncode = 1
    
    with patch('asyncio.to_thread', return_value=mock_result):
        result = await ytdlp_service.ensure_ytdlp_proxy_available()
        assert result is False


@pytest.mark.asyncio
async def test_ensure_ytdlp_proxy_available_exception(ytdlp_service):
    """Test yt-dlp-proxy availability check with exception."""
    with patch('asyncio.to_thread', side_effect=Exception("Test error")):
        result = await ytdlp_service.ensure_ytdlp_proxy_available()
        assert result is False


@pytest.mark.asyncio
async def test_download_with_proxy_success(ytdlp_service):
    """Test successful download with yt-dlp-proxy."""
    mock_result = MagicMock()
    mock_result.stdout = "Download completed"
    mock_result.stderr = ""
    
    with patch('asyncio.to_thread', return_value=mock_result):
        result = await ytdlp_service.download_with_proxy(
            url="https://example.com/video",
            output_template="/tmp/%(title)s.%(ext)s"
        )
        assert result == mock_result


@pytest.mark.asyncio
async def test_download_with_proxy_failure(ytdlp_service):
    """Test failed download with yt-dlp-proxy."""
    with patch('asyncio.to_thread', side_effect=subprocess.CalledProcessError(1, "cmd")):
        with pytest.raises(subprocess.CalledProcessError):
            await ytdlp_service.download_with_proxy(
                url="https://example.com/video",
                output_template="/tmp/%(title)s.%(ext)s"
            )


@pytest.mark.asyncio
async def test_run_update_script_success(ytdlp_service):
    """Test successful update script execution."""
    mock_result = MagicMock()
    mock_result.returncode = 0
    mock_result.stdout = "Update completed"
    mock_result.stderr = ""
    
    with patch('asyncio.to_thread', return_value=mock_result):
        result = await ytdlp_service.run_update_script(max_workers=4)
        assert result is True


@pytest.mark.asyncio
async def test_run_update_script_failure(ytdlp_service):
    """Test failed update script execution."""
    mock_result = MagicMock()
    mock_result.returncode = 1
    mock_result.stdout = ""
    mock_result.stderr = "Update failed"
    
    with patch('asyncio.to_thread', return_value=mock_result):
        result = await ytdlp_service.run_update_script(max_workers=4)
        assert result is False


@pytest.mark.asyncio
async def test_run_update_script_cooldown(ytdlp_service):
    """Test update script cooldown mechanism."""
    import time
    
    # Set last update attempt to recent time
    ytdlp_service._last_update_attempt = time.time()
    
    result = await ytdlp_service.run_update_script(max_workers=4)
    assert result is False


@pytest.mark.asyncio
async def test_download_with_fallback_update_success_first_try(ytdlp_service):
    """Test successful download on first try."""
    mock_result = MagicMock()
    mock_result.stdout = "Download completed"
    mock_result.stderr = ""
    
    with patch.object(ytdlp_service, 'download_with_proxy', return_value=mock_result):
        result = await ytdlp_service.download_with_fallback_update(
            url="https://example.com/video",
            output_template="/tmp/%(title)s.%(ext)s"
        )
        assert result == mock_result


@pytest.mark.asyncio
async def test_download_with_fallback_update_success_after_update(ytdlp_service):
    """Test successful download after update."""
    mock_result = MagicMock()
    mock_result.stdout = "Download completed"
    mock_result.stderr = ""
    
    with patch.object(ytdlp_service, 'download_with_proxy', side_effect=[
        Exception("First attempt failed"),
        mock_result
    ]):
        with patch.object(ytdlp_service, 'run_update_script', return_value=True):
            result = await ytdlp_service.download_with_fallback_update(
                url="https://example.com/video",
                output_template="/tmp/%(title)s.%(ext)s"
            )
            assert result == mock_result


@pytest.mark.asyncio
async def test_download_with_fallback_update_failure_after_update(ytdlp_service):
    """Test failed download even after update."""
    first_error = Exception("First attempt failed")
    second_error = Exception("Second attempt failed")
    
    with patch.object(ytdlp_service, 'download_with_proxy', side_effect=[
        first_error,
        second_error
    ]):
        with patch.object(ytdlp_service, 'run_update_script', return_value=True):
            with pytest.raises(Exception) as exc_info:
                await ytdlp_service.download_with_fallback_update(
                    url="https://example.com/video",
                    output_template="/tmp/%(title)s.%(ext)s"
                )
            assert exc_info.value == second_error


@pytest.mark.asyncio
async def test_download_with_fallback_update_failure_update_failed(ytdlp_service):
    """Test download failure when update also fails."""
    first_error = Exception("First attempt failed")
    
    with patch.object(ytdlp_service, 'download_with_proxy', side_effect=first_error):
        with patch.object(ytdlp_service, 'run_update_script', return_value=False):
            with pytest.raises(Exception) as exc_info:
                await ytdlp_service.download_with_fallback_update(
                    url="https://example.com/video",
                    output_template="/tmp/%(title)s.%(ext)s"
                )
            assert exc_info.value == first_error
