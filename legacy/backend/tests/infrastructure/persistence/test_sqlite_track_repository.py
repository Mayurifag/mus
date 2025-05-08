from datetime import datetime
from pathlib import Path

import pytest

from mus.domain.track import Track
from mus.infrastructure.persistence.sqlite_track_repository import SQLiteTrackRepository


@pytest.fixture
async def repository(tmp_path):
    db_path = tmp_path / "test.db"
    repo = SQLiteTrackRepository(str(db_path))
    await repo._init_db()
    return repo


@pytest.fixture
def track():
    return Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,
        file_path=Path("/test/path/track.mp3"),
        added_at=int(datetime(2024, 1, 1).timestamp()),
    )


@pytest.fixture
def track2():
    return Track(
        title="Another Track",
        artist="Another Artist",
        duration=240,
        file_path=Path("/test/path/track2.mp3"),
        added_at=int(datetime(2024, 1, 2).timestamp()),
    )


async def test_add_and_get_track(repository, track):
    await repository.add(track)
    tracks = await repository.get_all()
    assert len(tracks) == 1
    assert tracks[0].title == track.title
    assert tracks[0].artist == track.artist
    assert tracks[0].duration == track.duration
    assert tracks[0].file_path == track.file_path
    assert tracks[0].added_at == track.added_at


async def test_exists_by_path(repository, track):
    await repository.add(track)
    assert await repository.exists_by_path(track.file_path)
    assert not await repository.exists_by_path(Path("/test/path/nonexistent.mp3"))


async def test_search_by_title(repository, track):
    await repository.add(track)
    tracks = await repository.search_by_title("Test")
    assert len(tracks) == 1
    assert tracks[0].title == track.title

    tracks = await repository.search_by_title("Nonexistent")
    assert len(tracks) == 0


async def test_clear_all_tracks(repository, track, track2):
    # Add multiple tracks
    await repository.add(track)
    await repository.add(track2)

    # Verify tracks exist
    tracks = await repository.get_all()
    assert len(tracks) == 2
    assert await repository.exists_by_path(track.file_path)
    assert await repository.exists_by_path(track2.file_path)

    # Clear all tracks
    await repository.clear_all_tracks()

    # Verify tracks are gone
    tracks = await repository.get_all()
    assert len(tracks) == 0
    assert not await repository.exists_by_path(track.file_path)
    assert not await repository.exists_by_path(track2.file_path)
