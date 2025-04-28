from pathlib import Path

import pytest

from mus.domain.track import Track
from mus.infrastructure.persistence.sqlite_track_repository import SQLiteTrackRepository


@pytest.fixture
async def repository(tmp_path):
    db_path = tmp_path / "test.db"
    repo = SQLiteTrackRepository(str(db_path))
    await repo._create_table()
    return repo


@pytest.mark.asyncio
async def test_add_and_get_track(repository):
    track = Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,
        file_path=Path("/test/path.mp3"),
        added_at=1234567890,
    )

    await repository.add(track)
    tracks = await repository.get_all()

    assert len(tracks) == 1
    assert tracks[0].title == track.title
    assert tracks[0].artist == track.artist
    assert tracks[0].duration == track.duration
    assert tracks[0].file_path == track.file_path
    assert tracks[0].added_at == track.added_at


@pytest.mark.asyncio
async def test_exists_by_path(repository):
    file_path = Path("/test/path.mp3")
    assert not await repository.exists_by_path(file_path)

    track = Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,
        file_path=file_path,
        added_at=1234567890,
    )
    await repository.add(track)

    assert await repository.exists_by_path(file_path)


@pytest.mark.asyncio
async def test_search_by_title(repository):
    tracks = [
        Track(
            title="First Track",
            artist="Artist 1",
            duration=180,
            file_path=Path("/test/1.mp3"),
            added_at=1234567890,
        ),
        Track(
            title="Second Track",
            artist="Artist 2",
            duration=240,
            file_path=Path("/test/2.mp3"),
            added_at=1234567891,
        ),
        Track(
            title="Third Track",
            artist="Artist 3",
            duration=300,
            file_path=Path("/test/3.mp3"),
            added_at=1234567892,
        ),
    ]

    for track in tracks:
        await repository.add(track)

    results = await repository.search_by_title("Track")
    assert len(results) == 3

    results = await repository.search_by_title("First")
    assert len(results) == 1
    assert results[0].title == "First Track"
