import pytest
import pytest_asyncio
from sqlmodel import SQLModel
from sqlalchemy.ext.asyncio import create_async_engine
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.track import Track
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


# Create a test database engine
TEST_DATABASE_URL = "sqlite+aiosqlite:///:memory:"
engine = create_async_engine(TEST_DATABASE_URL)


@pytest_asyncio.fixture
async def session():
    async with AsyncSession(engine) as session:
        yield session


@pytest_asyncio.fixture
async def setup_test_db():
    # Create tables
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all)
    yield
    # Tear down - remove tables
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.drop_all)


@pytest_asyncio.fixture
async def repository(session):
    return SQLiteTrackRepository(session)


@pytest_asyncio.fixture
async def sample_track():
    return Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,  # 3 minutes
        file_path="/path/to/test.mp3",
        has_cover=False,
    )


@pytest.mark.asyncio
async def test_add_track(setup_test_db, repository, sample_track):
    """Test adding a new track."""
    # Add the track
    added_track = await repository.add(sample_track)

    # Check that it was added with an ID
    assert added_track.id is not None
    assert added_track.title == "Test Track"
    assert added_track.artist == "Test Artist"
    assert added_track.duration == 180
    assert added_track.file_path == "/path/to/test.mp3"
    assert added_track.has_cover is False

    # Check that we can retrieve it
    retrieved_track = await repository.get_by_id(added_track.id)
    assert retrieved_track is not None
    assert retrieved_track.id == added_track.id
    assert retrieved_track.title == "Test Track"


@pytest.mark.asyncio
async def test_get_all_tracks(setup_test_db, repository, sample_track):
    """Test retrieving all tracks."""
    # Add a few tracks
    track1 = await repository.add(sample_track)

    track2 = Track(
        title="Another Track",
        artist="Another Artist",
        duration=240,
        file_path="/path/to/another.mp3",
        has_cover=True,
    )
    track2 = await repository.add(track2)

    # Get all tracks
    tracks = await repository.get_all()

    # Check that both tracks are returned
    assert len(tracks) == 2
    track_ids = {track.id for track in tracks}
    assert track1.id in track_ids
    assert track2.id in track_ids


@pytest.mark.asyncio
async def test_exists_by_path(setup_test_db, repository, sample_track):
    """Test checking if a track exists by file path."""
    # Add a track
    await repository.add(sample_track)

    # Check that it exists
    exists = await repository.exists_by_path(sample_track.file_path)
    assert exists is True

    # Check that a non-existent track doesn't exist
    exists = await repository.exists_by_path("/path/to/nonexistent.mp3")
    assert exists is False


@pytest.mark.asyncio
async def test_set_cover_flag(setup_test_db, session, sample_track):
    """Test setting the has_cover flag."""
    repository = SQLiteTrackRepository(session)

    # Add a track
    track = await repository.add(sample_track)
    track_id = track.id
    assert track.has_cover is False

    # Set has_cover to True
    await repository.set_cover_flag(track_id, True)
    await session.commit()

    # Create a new repository to verify
    new_repository = SQLiteTrackRepository(session)

    # Get the track using the repository's get_by_id method
    new_track = await new_repository.get_by_id(track_id)

    # Check that the flag was set
    assert new_track is not None
    assert new_track.has_cover is True


@pytest.mark.asyncio
async def test_upsert_track_insert_new(setup_test_db, repository, sample_track):
    """Test upserting a new track."""
    # Upsert a new track
    track = await repository.upsert_track(sample_track)

    # Check that it was inserted with an ID
    assert track.id is not None
    assert track.title == "Test Track"
    assert track.artist == "Test Artist"

    # Verify with a separate query
    all_tracks = await repository.get_all()
    assert len(all_tracks) == 1
    assert all_tracks[0].id == track.id


@pytest.mark.asyncio
async def test_upsert_track_update_existing(setup_test_db, repository, sample_track):
    """Test upserting a track that already exists."""
    # First add the track
    original_track = await repository.add(sample_track)
    original_id = original_track.id
    original_added_at = original_track.added_at

    # Now update it via upsert with same file_path but different metadata
    updated_track_data = Track(
        title="Updated Title",
        artist="Updated Artist",
        duration=200,
        file_path=sample_track.file_path,  # Same file path
        has_cover=True,  # Changed
    )

    updated_track = await repository.upsert_track(updated_track_data)

    # Check that metadata was updated but ID and added_at preserved
    assert updated_track.id == original_id  # Same ID
    assert updated_track.added_at == original_added_at  # Same added_at timestamp
    assert updated_track.title == "Updated Title"  # Updated title
    assert updated_track.artist == "Updated Artist"  # Updated artist
    assert updated_track.duration == 200  # Updated duration
    assert updated_track.has_cover is True  # Updated has_cover

    # Verify only one track exists (no duplicates)
    all_tracks = await repository.get_all()
    assert len(all_tracks) == 1
