import pytest
import pytest_asyncio


from src.mus.domain.entities.player_state import PlayerState


@pytest_asyncio.fixture
async def repository(player_state_repository):
    return player_state_repository


@pytest.mark.asyncio
async def test_save_and_load_state(repository, sample_state):
    """Test saving and loading player state."""
    # Save the state
    saved_state = await repository.save_state(sample_state)

    # Check that it was saved with ID=1
    assert saved_state.id == 1
    assert saved_state.current_track_id == 42
    assert saved_state.progress_seconds == 30
    assert saved_state.volume_level == 0.8
    assert saved_state.is_muted is False

    # Load the state and verify
    loaded_state = await repository.load_state()
    assert loaded_state is not None
    assert loaded_state.id == 1
    assert loaded_state.current_track_id == 42
    assert loaded_state.progress_seconds == 30
    assert loaded_state.volume_level == 0.8
    assert loaded_state.is_muted is False


@pytest.mark.asyncio
async def test_update_existing_state(repository, sample_state):
    """Test updating an existing player state."""
    # First save the initial state
    await repository.save_state(sample_state)

    # Now update it with new values
    updated_state = PlayerState(
        current_track_id=43,  # Changed
        progress_seconds=60,  # Changed
        volume_level=0.5,  # Changed
        is_muted=True,  # Changed
    )

    saved_state = await repository.save_state(updated_state)

    # Check that the state was updated
    assert saved_state.id == 1  # Same ID
    assert saved_state.current_track_id == 43
    assert saved_state.progress_seconds == 60
    assert saved_state.volume_level == 0.5
    assert saved_state.is_muted is True

    # Verify by loading
    loaded_state = await repository.load_state()
    assert loaded_state is not None
    assert loaded_state.id == 1
    assert loaded_state.current_track_id == 43
    assert loaded_state.progress_seconds == 60
    assert loaded_state.volume_level == 0.5
    assert loaded_state.is_muted is True


@pytest.mark.asyncio
async def test_load_nonexistent_state(repository):
    """Test loading a state that doesn't exist yet."""
    state = await repository.load_state()
    assert state is None
