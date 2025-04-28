import pytest
from fastapi.testclient import TestClient

from mus.infrastructure.web.main import app

client = TestClient(app)


@pytest.mark.skip(reason="Infrastructure layer not implemented yet")
def test_root_route():
    """Test the root route."""
    response = client.get("/")
    assert response.status_code == 200
