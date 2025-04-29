from datetime import datetime
from pathlib import Path

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


async def test_get_initial_state_with_valid_state(track1):
    state = PlayerState(
        current_track_id=1, progress_seconds=30.5, volume_level=0.75, is_muted=True
    )

    service = InitialStateService(
        load_player_state_use_case=MockLoadPlayerStateUseCase(state),
        track_repository=MockTrackRepository([track1]),
    )

    result = await service.get_initial_state()
    assert result["player_state"]["current_track_id"] == state.current_track_id
    assert result["player_state"]["progress_seconds"] == state.progress_seconds
    assert result["player_state"]["volume_level"] == state.volume_level
    assert result["player_state"]["is_muted"] == state.is_muted
    assert len(result["tracks"]) == 1
    assert result["tracks"][0].id == track1.id


async def test_get_initial_state_with_nonexistent_track(tracks):
    state = PlayerState(
        current_track_id=999,  # Non-existent track ID
        progress_seconds=30.5,
        volume_level=0.75,
        is_muted=True,
    )

    service = InitialStateService(
        load_player_state_use_case=MockLoadPlayerStateUseCase(state),
        track_repository=MockTrackRepository(tracks),
    )

    result = await service.get_initial_state()
    assert result["player_state"]["current_track_id"] == tracks[0].id
    assert result["player_state"]["progress_seconds"] == 0.0
    assert result["player_state"]["volume_level"] == 1.0
    assert result["player_state"]["is_muted"] is False
    assert len(result["tracks"]) == 2


async def test_get_initial_state_with_no_state():
    # Create mock implementations
    load_player_state = MockLoadPlayerStateUseCase(None)
    track_repository = MockTrackRepository([])

    service = InitialStateService(load_player_state, track_repository)
    initial_state = await service.get_initial_state()

    assert initial_state == {
        "tracks": [],
        "player_state": {
            "current_track_id": None,
            "progress_seconds": 0.0,
            "volume_level": 1.0,
            "is_muted": False,
        },
    }


async def test_get_initial_state_with_no_tracks():
    state = PlayerState(
        current_track_id=1, progress_seconds=30.5, volume_level=0.75, is_muted=True
    )

    service = InitialStateService(
        load_player_state_use_case=MockLoadPlayerStateUseCase(state),
        track_repository=MockTrackRepository([]),
    )

    result = await service.get_initial_state()
    assert result["player_state"]["current_track_id"] is None
    assert result["player_state"]["progress_seconds"] == 0.0
    assert result["player_state"]["volume_level"] == 1.0
    assert result["player_state"]["is_muted"] is False
    assert len(result["tracks"]) == 0
