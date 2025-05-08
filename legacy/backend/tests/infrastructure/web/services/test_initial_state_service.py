from datetime import datetime
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock

import pytest

from mus.application.ports.track_repository import ITrackRepository
from mus.application.use_cases.load_player_state import LoadPlayerStateUseCase
from mus.domain.player_state import PlayerState
from mus.domain.track import Track
from mus.infrastructure.web.services.initial_state_service import InitialStateService


class MockLoadPlayerStateUseCase(LoadPlayerStateUseCase):
    def __init__(self, state: PlayerState | None = None):
        self._state = state

    async def execute(self) -> PlayerState | None:
        return self._state


class MockTrackRepository(ITrackRepository):
    def __init__(self, tracks: list[Track]):
        self._tracks = tracks

    async def get_by_id(self, track_id: str) -> Track | None:
        return next((t for t in self._tracks if t.id == track_id), None)

    async def get_all(self) -> list[Track]:
        return self._tracks

    async def add(self, track: Track) -> None:
        self._tracks.append(track)

    async def exists_by_path(self, path: str) -> bool:
        return any(t.file_path == path for t in self._tracks)

    async def search_by_title(self, title: str) -> list[Track]:
        return [t for t in self._tracks if title.lower() in t.title.lower()]

    async def clear_all_tracks(self) -> None:
        self._tracks.clear()


@pytest.fixture
def track1():
    return Track(
        id=1,
        title="Track 1",
        artist="Artist 1",
        file_path=Path("path1.mp3"),
        duration=180,
        added_at=int(datetime.now().timestamp()),
    )


@pytest.fixture
def track2():
    return Track(
        id=2,
        title="Track 2",
        artist="Artist 2",
        file_path=Path("path2.mp3"),
        duration=240,
        added_at=int(datetime.now().timestamp()),
    )


@pytest.fixture
def tracks(track1, track2):
    return [track1, track2]


@pytest.fixture
def load_player_state_use_case():
    use_case = MagicMock()
    use_case.execute = AsyncMock()
    return use_case


@pytest.fixture
def track_repository():
    repo = MagicMock()
    repo.get_all = AsyncMock()
    return repo


@pytest.fixture
def initial_state_service(load_player_state_use_case, track_repository):
    return InitialStateService(load_player_state_use_case, track_repository)


async def test_get_initial_state_with_valid_state(initial_state_service):
    # Setup test data
    track1 = Track(
        id=1,
        title="Test Track 1",
        artist="Test Artist 1",
        duration=180,
        file_path=Path("/test/path1.mp3"),
        added_at=1234567890,
        has_cover=True,
    )
    track2 = Track(
        id=2,
        title="Test Track 2",
        artist="Test Artist 2",
        duration=240,
        file_path=Path("/test/path2.mp3"),
        added_at=1234567891,
        has_cover=False,
    )

    initial_state_service._track_repository.get_all.return_value = [track1, track2]
    initial_state_service._load_player_state.execute.return_value = PlayerState(
        current_track_id=1,
        progress_seconds=30.5,
        volume_level=0.8,
        is_muted=False,
    )

    # Execute
    result = await initial_state_service.get_initial_state()

    # Verify
    assert result["tracks"][0]["id"] == track1.id
    assert result["tracks"][0]["title"] == track1.title
    assert result["tracks"][0]["artist"] == track1.artist
    assert result["tracks"][0]["has_cover"] is True
    assert result["tracks"][0]["cover_small_url"] == "/covers/small/1.webp"
    assert result["tracks"][0]["cover_medium_url"] == "/covers/medium/1.webp"

    assert result["tracks"][1]["id"] == track2.id
    assert result["tracks"][1]["title"] == track2.title
    assert result["tracks"][1]["artist"] == track2.artist
    assert result["tracks"][1]["has_cover"] is False
    assert result["tracks"][1]["cover_small_url"] == "/static/images/placeholder.svg"
    assert result["tracks"][1]["cover_medium_url"] == "/static/images/placeholder.svg"

    assert result["player_state"]["current_track_id"] == 1
    assert result["player_state"]["progress_seconds"] == 30.5
    assert result["player_state"]["volume_level"] == 0.8
    assert result["player_state"]["is_muted"] is False


async def test_get_initial_state_with_nonexistent_track(initial_state_service):
    # Setup test data
    track1 = Track(
        id=1,
        title="Test Track 1",
        artist="Test Artist 1",
        duration=180,
        file_path=Path("/test/path1.mp3"),
        added_at=1234567890,
    )

    initial_state_service._track_repository.get_all.return_value = [track1]
    initial_state_service._load_player_state.execute.return_value = PlayerState(
        current_track_id=2,  # Track ID that doesn't exist
        progress_seconds=30.5,
        volume_level=0.8,
        is_muted=False,
    )

    # Execute
    result = await initial_state_service.get_initial_state()

    # Verify
    assert (
        result["player_state"]["current_track_id"] == 1
    )  # Should default to first track
    assert result["player_state"]["progress_seconds"] == 0.0  # Should reset progress
    assert result["player_state"]["volume_level"] == 1.0  # Should use default volume
    assert result["player_state"]["is_muted"] is False


async def test_get_initial_state_with_no_state(initial_state_service):
    # Setup test data
    track1 = Track(
        id=1,
        title="Test Track 1",
        artist="Test Artist 1",
        duration=180,
        file_path=Path("/test/path1.mp3"),
        added_at=1234567890,
    )

    initial_state_service._track_repository.get_all.return_value = [track1]
    initial_state_service._load_player_state.execute.return_value = None

    # Execute
    result = await initial_state_service.get_initial_state()

    # Verify
    assert (
        result["player_state"]["current_track_id"] == 1
    )  # Should default to first track
    assert result["player_state"]["progress_seconds"] == 0.0
    assert result["player_state"]["volume_level"] == 1.0
    assert result["player_state"]["is_muted"] is False


async def test_get_initial_state_with_no_tracks(initial_state_service):
    # Setup test data
    initial_state_service._track_repository.get_all.return_value = []
    initial_state_service._load_player_state.execute.return_value = None

    # Execute
    result = await initial_state_service.get_initial_state()

    # Verify
    assert len(result["tracks"]) == 0
    assert result["player_state"]["current_track_id"] is None
    assert result["player_state"]["progress_seconds"] == 0.0
    assert result["player_state"]["volume_level"] == 1.0
    assert result["player_state"]["is_muted"] is False
