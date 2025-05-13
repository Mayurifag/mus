import pytest
from unittest.mock import AsyncMock

from src.mus.application.use_cases.manage_player_state_use_case import (
    ManagePlayerStateUseCase,
)
from src.mus.application.dtos.player_state import PlayerStateDTO
from src.mus.domain.entities.player_state import PlayerState
from src.mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)


@pytest.fixture
def mock_repository():
    repository = AsyncMock(spec=SQLitePlayerStateRepository)
    return repository


@pytest.fixture
def use_case(mock_repository):
    return ManagePlayerStateUseCase(repository=mock_repository)


@pytest.mark.asyncio
async def test_save_state(use_case, mock_repository):
    # Create DTO to save
    state_dto = PlayerStateDTO(
        current_track_id=42,
        progress_seconds=30.5,
        volume_level=0.8,
        is_muted=False,
    )

    # Setup mock to return a saved state
    saved_state = PlayerState(
        id=1,
        current_track_id=state_dto.current_track_id,
        progress_seconds=state_dto.progress_seconds,
        volume_level=state_dto.volume_level,
        is_muted=state_dto.is_muted,
    )
    mock_repository.save_state.return_value = saved_state

    # Call the use case
    result = await use_case.save_state(state_dto)

    # Verify the result
    assert isinstance(result, PlayerStateDTO)
    assert result.current_track_id == state_dto.current_track_id
    assert result.progress_seconds == state_dto.progress_seconds
    assert result.volume_level == state_dto.volume_level
    assert result.is_muted == state_dto.is_muted

    # Verify the repository was called correctly
    mock_repository.save_state.assert_called_once()
    call_args = mock_repository.save_state.call_args[0][0]
    assert isinstance(call_args, PlayerState)
    assert call_args.current_track_id == state_dto.current_track_id
    assert call_args.progress_seconds == state_dto.progress_seconds
    assert call_args.volume_level == state_dto.volume_level
    assert call_args.is_muted == state_dto.is_muted


@pytest.mark.asyncio
async def test_load_state_exists(use_case, mock_repository):
    # Setup mock to return an existing state
    existing_state = PlayerState(
        id=1,
        current_track_id=42,
        progress_seconds=30.5,
        volume_level=0.8,
        is_muted=False,
    )
    mock_repository.load_state.return_value = existing_state

    # Call the use case
    result = await use_case.load_state()

    # Verify the result
    assert isinstance(result, PlayerStateDTO)
    assert result.current_track_id == existing_state.current_track_id
    assert result.progress_seconds == existing_state.progress_seconds
    assert result.volume_level == existing_state.volume_level
    assert result.is_muted == existing_state.is_muted

    # Verify the repository was called
    mock_repository.load_state.assert_called_once()


@pytest.mark.asyncio
async def test_load_state_not_exists(use_case, mock_repository):
    # Setup mock to return None (no state exists)
    mock_repository.load_state.return_value = None

    # Call the use case
    result = await use_case.load_state()

    # Verify default values are returned
    assert isinstance(result, PlayerStateDTO)
    assert result.current_track_id is None
    assert result.progress_seconds == 0.0
    assert result.volume_level == 1.0
    assert result.is_muted is False

    # Verify the repository was called
    mock_repository.load_state.assert_called_once()
