import pytest
import pytest_asyncio
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.track import Track
from src.mus.domain.entities.track_history import TrackHistory
from src.mus.infrastructure.persistence.sqlite_track_history_repository import (
    SQLiteTrackHistoryRepository,
)

warning_filter = "ignore::sqlalchemy.exc.SAWarning"


@pytest_asyncio.fixture
async def sample_track(session: AsyncSession) -> Track:
    track = Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,
        file_path="/path/to/test.mp3",
        added_at=1609459200,
    )
    session.add(track)
    await session.commit()
    await session.refresh(track)
    return track


@pytest_asyncio.fixture
async def repository(session: AsyncSession) -> SQLiteTrackHistoryRepository:
    return SQLiteTrackHistoryRepository(session)


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_add_history_entry(repository, sample_track):
    track_id = sample_track.id
    history_entry = TrackHistory(
        track_id=track_id,
        title="Old Title",
        artist="Old Artist",
        duration=120,
        changed_at=1609459100,
    )

    result = await repository.add(history_entry)

    assert result.id is not None
    assert result.track_id == track_id
    assert result.title == "Old Title"
    assert result.artist == "Old Artist"
    assert result.duration == 120
    assert result.changed_at == 1609459100


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_get_by_track_id(repository, sample_track):
    track_id = sample_track.id
    history1 = TrackHistory(
        track_id=track_id,
        title="Old Title 1",
        artist="Old Artist 1",
        duration=120,
        changed_at=1609459100,
    )
    history2 = TrackHistory(
        track_id=track_id,
        title="Old Title 2",
        artist="Old Artist 2",
        duration=130,
        changed_at=1609459200,
    )

    await repository.add(history1)
    await repository.add(history2)

    results = await repository.get_by_track_id(track_id)

    assert len(results) == 2
    assert results[0].changed_at == 1609459200
    assert results[1].changed_at == 1609459100


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_get_by_id(repository, sample_track):
    history_entry = TrackHistory(
        track_id=sample_track.id,
        title="Old Title",
        artist="Old Artist",
        duration=120,
        changed_at=1609459100,
    )

    added_entry = await repository.add(history_entry)
    result = await repository.get_by_id(added_entry.id)

    assert result is not None
    assert result.id == added_entry.id
    assert result.title == "Old Title"


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_get_by_id_not_found(repository):
    result = await repository.get_by_id(999)
    assert result is None


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_prune_history(repository, sample_track):
    track_id = sample_track.id
    for i in range(7):
        history_entry = TrackHistory(
            track_id=track_id,
            title=f"Old Title {i}",
            artist=f"Old Artist {i}",
            duration=120 + i,
            changed_at=1609459100 + i,
        )
        await repository.add(history_entry)

    await repository.prune_history(track_id, 3)

    results = await repository.get_by_track_id(track_id)
    assert len(results) == 3
    assert results[0].changed_at == 1609459106
    assert results[1].changed_at == 1609459105
    assert results[2].changed_at == 1609459104
