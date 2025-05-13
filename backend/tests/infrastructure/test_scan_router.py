import pytest
import httpx
from unittest.mock import patch, AsyncMock
from src.mus.application.dtos.scan import ScanResponseDTO


@pytest.mark.asyncio
async def test_scan_directory_success(app):
    """Test successful scanning of music directory."""
    # Mock the scan_directory method of ScanTracksUseCase
    mock_scan_response = ScanResponseDTO(
        success=True,
        message="Scan completed: 5 added, 2 updated, 0 errors",
        tracks_added=5,
        tracks_updated=2,
        errors=0,
    )

    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.ScanTracksUseCase.scan_directory",
        new_callable=AsyncMock,
        return_value=mock_scan_response,
    ):
        async with httpx.AsyncClient(
            transport=httpx.ASGITransport(app=app), base_url="http://test"
        ) as ac:
            response = await ac.post("/api/v1/scan", json={})

            assert response.status_code == 200
            result = response.json()
            assert result["success"] is True
            assert result["tracks_added"] == 5
            assert result["tracks_updated"] == 2
            assert result["errors"] == 0


@pytest.mark.asyncio
async def test_scan_directory_with_custom_paths(app):
    """Test scanning with custom directory paths."""
    custom_paths = ["/custom/path1", "/custom/path2"]

    # Mock to verify the paths are passed correctly
    mock_scan_directory = AsyncMock(
        return_value=ScanResponseDTO(
            success=True,
            message="Scan completed: 3 added, 0 updated, 0 errors",
            tracks_added=3,
            tracks_updated=0,
            errors=0,
        )
    )

    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.ScanTracksUseCase.scan_directory",
        mock_scan_directory,
    ):
        async with httpx.AsyncClient(
            transport=httpx.ASGITransport(app=app), base_url="http://test"
        ) as ac:
            response = await ac.post(
                "/api/v1/scan", json={"directory_paths": custom_paths}
            )

            assert response.status_code == 200
            result = response.json()
            assert result["success"] is True

            # Verify that the custom paths were passed to the use case
            mock_scan_directory.assert_called_once_with(custom_paths)


@pytest.mark.asyncio
async def test_scan_directory_with_errors(app):
    """Test scanning that encounters errors."""
    error_details = ["Error processing file1.mp3", "Error processing file2.mp3"]
    mock_scan_response = ScanResponseDTO(
        success=True,  # Overall process still succeeds but with errors
        message="Scan completed: 3 added, 0 updated, 2 errors",
        tracks_added=3,
        tracks_updated=0,
        errors=2,
        error_details=error_details,
    )

    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.ScanTracksUseCase.scan_directory",
        new_callable=AsyncMock,
        return_value=mock_scan_response,
    ):
        async with httpx.AsyncClient(
            transport=httpx.ASGITransport(app=app), base_url="http://test"
        ) as ac:
            response = await ac.post("/api/v1/scan", json={})

            assert response.status_code == 200
            result = response.json()
            assert result["success"] is True
            assert result["tracks_added"] == 3
            assert result["errors"] == 2
            assert result["error_details"] == error_details


@pytest.mark.asyncio
async def test_scan_directory_exception(app):
    """Test handling of exceptions during scanning."""
    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.ScanTracksUseCase.scan_directory",
        side_effect=Exception("Unexpected error during scan"),
    ):
        async with httpx.AsyncClient(
            transport=httpx.ASGITransport(app=app), base_url="http://test"
        ) as ac:
            response = await ac.post("/api/v1/scan", json={})

            # We now expect a 200 status code but with success=False
            assert response.status_code == 200
            result = response.json()
            assert result["success"] is False
            assert result["errors"] == 1
            assert "Unexpected error during scan" in result["message"]
            assert len(result["error_details"]) == 1
            assert "Unexpected error during scan" in result["error_details"][0]
