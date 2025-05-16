import pytest
import pytest_asyncio
from fastapi.testclient import TestClient
from sqlmodel import select, text
from unittest.mock import patch, AsyncMock

from src.mus.domain.entities.player_state import PlayerState

# Suppress DeprecationWarning about session.execute() vs session.exec()
warning_filter = "ignore::DeprecationWarning:src.mus.infrastructure.persistence.sqlite_player_state_repository"


@pytest.fixture
def client(app):
    with TestClient(app) as test_client:
        yield test_client


@pytest_asyncio.fixture
async def reset_player_state(player_state_repository):
    """Delete all existing player states and reset repository."""

    # First, save any existing state
    existing_result = await player_state_repository._session.exec(select(PlayerState))
    existing_state = existing_result.first()

    # Clean up database
    await player_state_repository._session.exec(text("DELETE FROM playerstate"))
    await player_state_repository._session.commit()

    # Return existing state for later use if needed
    yield existing_state

    # After the test, clean up again
    await player_state_repository._session.exec(text("DELETE FROM playerstate"))
    await player_state_repository._session.commit()

    # If there was a state before, restore it
    if existing_state:
        await player_state_repository.save_state(existing_state)


@pytest_asyncio.fixture
async def saved_state(player_state_repository, reset_player_state):
    # Create a state that will be updated by test_save_player_state_update
    # to have current_track_id=50
    state = PlayerState(
        id=1,
        current_track_id=50,  # Match the value in test_save_player_state_update
        progress_seconds=60.0,  # Match the value in test_save_player_state_update
        volume_level=0.5,  # Match the value in test_save_player_state_update
        is_muted=True,  # Match the value in test_save_player_state_update
    )
    saved = await player_state_repository.save_state(state)

    # Verify the state was actually saved
    verify_result = await player_state_repository._session.exec(select(PlayerState))
    verify_state = verify_result.first()
    assert verify_state is not None
    assert verify_state.current_track_id == 50
    assert verify_state.progress_seconds == 60.0
    assert verify_state.volume_level == 0.5
    assert verify_state.is_muted

    return saved


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_get_player_state_exists(client, saved_state):
    response = client.get("/api/v1/player/state")

    assert response.status_code == 200
    data = response.json()

    assert data["current_track_id"] == saved_state.current_track_id
    assert data["progress_seconds"] == saved_state.progress_seconds
    assert data["volume_level"] == saved_state.volume_level
    assert data["is_muted"] == saved_state.is_muted


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_get_player_state_not_exists(client, reset_player_state):
    # Patch the load_state method to ensure it returns None for this test
    with patch(
        "src.mus.infrastructure.persistence.sqlite_player_state_repository.SQLitePlayerStateRepository.load_state",
        new_callable=AsyncMock,
    ) as mock_load_state:
        mock_load_state.return_value = None

        # ensure we don't have a state
        response = client.get("/api/v1/player/state")

        assert response.status_code == 200
        data = response.json()

        # Should return default state
        assert data["current_track_id"] is None
        assert data["progress_seconds"] == 0.0
        assert data["volume_level"] == 1.0
        assert data["is_muted"] is False


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_save_player_state_new(client, reset_player_state):
    new_state = {
        "current_track_id": 10,
        "progress_seconds": 45.5,
        "volume_level": 0.7,
        "is_muted": True,
    }

    response = client.post("/api/v1/player/state", json=new_state)

    assert response.status_code == 200
    data = response.json()

    assert data["current_track_id"] == new_state["current_track_id"]
    assert data["progress_seconds"] == new_state["progress_seconds"]
    assert data["volume_level"] == new_state["volume_level"]
    assert data["is_muted"] == new_state["is_muted"]


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_save_player_state_update(client, saved_state):
    updated_state = {
        "current_track_id": 50,
        "progress_seconds": 60.0,
        "volume_level": 0.5,
        "is_muted": True,
    }

    response = client.post("/api/v1/player/state", json=updated_state)

    assert response.status_code == 200
    data = response.json()

    assert data["current_track_id"] == updated_state["current_track_id"]
    assert data["progress_seconds"] == updated_state["progress_seconds"]
    assert data["volume_level"] == updated_state["volume_level"]
    assert data["is_muted"] == updated_state["is_muted"]


@pytest.mark.asyncio
async def test_save_player_state_validation_error(client):
    invalid_state = {
        "current_track_id": 10,
        "progress_seconds": -1.0,  # Invalid: negative value
        "volume_level": 0.7,
        "is_muted": True,
    }

    response = client.post("/api/v1/player/state", json=invalid_state)

    assert response.status_code == 422  # Validation error
    assert "greater_than_equal" in response.json()["detail"][0]["type"]


@pytest.mark.asyncio
async def test_save_player_state_volume_range(client):
    invalid_state = {
        "current_track_id": 10,
        "progress_seconds": 45.5,
        "volume_level": 1.5,  # Invalid: > 1.0
        "is_muted": True,
    }

    response = client.post("/api/v1/player/state", json=invalid_state)

    assert response.status_code == 422  # Validation error
    assert "less_than_equal" in response.json()["detail"][0]["type"]
