import os

import pytest
from fastapi.testclient import TestClient

from mus.infrastructure.web.main import app


@pytest.fixture
def test_db(tmp_path):
    db_path = str(tmp_path / "test.db")
    os.environ["DATABASE_PATH"] = db_path
    yield db_path
    if os.path.exists(db_path):
        os.remove(db_path)


@pytest.fixture
def client(test_db):
    return TestClient(app)


async def test_save_state(client):
    state = {
        "current_track_id": 1,
        "progress_seconds": 30.5,
        "volume_level": 0.75,
        "is_muted": True,
    }

    response = client.post("/state", json=state)
    assert response.status_code == 200


async def test_save_state_with_null_track_id(client):
    state = {
        "current_track_id": None,
        "progress_seconds": 0.0,
        "volume_level": 1.0,
        "is_muted": False,
    }

    response = client.post("/state", json=state)
    assert response.status_code == 200


async def test_save_state_with_invalid_data(client):
    state = {
        "current_track_id": "invalid",  # Should be int or None
        "progress_seconds": "invalid",  # Should be float
        "volume_level": "invalid",  # Should be float
        "is_muted": "invalid",  # Should be bool
    }

    response = client.post("/state", json=state)
    assert response.status_code == 422  # Validation error
