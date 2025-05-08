import pytest

from mus.domain.player_state import PlayerState
from mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)


@pytest.fixture
def db_path(tmp_path):
    return str(tmp_path / "test.db")


@pytest.fixture
def repository(db_path):
    return SQLitePlayerStateRepository(db_path)


async def test_save_and_load_state(repository):
    state = PlayerState(
        current_track_id=1, progress_seconds=30.5, volume_level=0.75, is_muted=True
    )

    await repository.save_state(state)
    loaded_state = await repository.load_state()

    assert loaded_state is not None
    assert loaded_state.current_track_id == state.current_track_id
    assert loaded_state.progress_seconds == state.progress_seconds
    assert loaded_state.volume_level == state.volume_level
    assert loaded_state.is_muted == state.is_muted


async def test_load_nonexistent_state(repository):
    state = await repository.load_state()
    assert state is None


async def test_save_null_track_id(repository):
    state = PlayerState(
        current_track_id=None, progress_seconds=0.0, volume_level=1.0, is_muted=False
    )

    await repository.save_state(state)
    loaded_state = await repository.load_state()

    assert loaded_state is not None
    assert loaded_state.current_track_id is None


async def test_update_existing_state(repository):
    initial_state = PlayerState(
        current_track_id=1, progress_seconds=30.5, volume_level=0.75, is_muted=True
    )

    updated_state = PlayerState(
        current_track_id=2, progress_seconds=45.0, volume_level=0.5, is_muted=False
    )

    await repository.save_state(initial_state)
    await repository.save_state(updated_state)

    loaded_state = await repository.load_state()
    assert loaded_state is not None
    assert loaded_state.current_track_id == updated_state.current_track_id
    assert loaded_state.progress_seconds == updated_state.progress_seconds
    assert loaded_state.volume_level == updated_state.volume_level
    assert loaded_state.is_muted == updated_state.is_muted
