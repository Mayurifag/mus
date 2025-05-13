import pytest
from fastapi.testclient import TestClient
from unittest.mock import patch, AsyncMock

from src.mus.main import app
from src.mus.application.dtos.scan import ScanResponseDTO
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase


@pytest.fixture
def client():
    with TestClient(app) as test_client:
        yield test_client


@pytest.mark.asyncio
async def test_scan_endpoint_success(client):
    success_response = ScanResponseDTO(
        success=True,
        message="Scan completed: 5 added, 2 updated, 0 errors",
        tracks_added=5,
        tracks_updated=2,
        errors=0,
    )

    with patch.object(
        ScanTracksUseCase, "scan_directory", new_callable=AsyncMock
    ) as mock_scan:
        mock_scan.return_value = success_response

        response = client.post("/api/v1/scan", json={})

        assert response.status_code == 200
        data = response.json()

        assert data["success"] is True
        assert data["tracks_added"] == 5
        assert data["tracks_updated"] == 2
        assert data["errors"] == 0
        assert "Scan completed" in data["message"]
        assert mock_scan.called


@pytest.mark.asyncio
async def test_scan_endpoint_with_custom_path(client):
    success_response = ScanResponseDTO(
        success=True,
        message="Scan completed: 3 added, 0 updated, 0 errors",
        tracks_added=3,
        tracks_updated=0,
        errors=0,
    )

    custom_directory = ["/custom/music/path"]

    with patch.object(
        ScanTracksUseCase, "scan_directory", new_callable=AsyncMock
    ) as mock_scan:
        mock_scan.return_value = success_response

        response = client.post(
            "/api/v1/scan", json={"directory_paths": custom_directory}
        )

        assert response.status_code == 200
        mock_scan.assert_called_once_with(custom_directory)


@pytest.mark.asyncio
async def test_scan_endpoint_error(client):
    with patch.object(
        ScanTracksUseCase, "scan_directory", new_callable=AsyncMock
    ) as mock_scan:
        mock_scan.side_effect = Exception("Permission denied: /path/to/music")

        response = client.post("/api/v1/scan", json={})

        assert response.status_code == 200  # Note: The API returns 200 even for errors
        data = response.json()

        assert data["success"] is False
        assert data["tracks_added"] == 0
        assert data["errors"] == 1
        assert "Error during scan" in data["message"]
        assert len(data["error_details"]) == 1


@pytest.mark.asyncio
async def test_scan_endpoint_validation_error(client):
    # Invalid type for directory_paths (should be array)
    invalid_request = {"directory_paths": "not_an_array"}

    response = client.post("/api/v1/scan", json=invalid_request)

    assert response.status_code == 422  # Validation error
    assert "list_type" in response.json()["detail"][0]["type"]
