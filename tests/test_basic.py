import pytest
from fastapi.testclient import TestClient

from mus.infrastructure.web.main import app


@pytest.mark.asyncio
async def test_root_route():
    """Test the root route."""
    client = TestClient(app)
    response = client.get("/")
    assert response.status_code == 200
