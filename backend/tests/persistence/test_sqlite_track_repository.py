import pytest
import pytest_asyncio
from src.mus.domain.entities.track import Track
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


# Suppress DeprecationWarning about session.execute() vs session.exec()
warning_filter = "ignore::DeprecationWarning:src.mus.infrastructure.persistence.sqlite_track_repository"


@pytest_asyncio.fixture
async def repository(track_repository):
    return track_repository


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_add_track(repository, sample_track):
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
@pytest.mark.filterwarnings(warning_filter)
async def test_get_all_tracks(repository, sample_track):
    # Add a few tracks
    track1 = await repository.add(sample_track)
    track1_id = track1.id

    track2 = Track(
        title="Another Track",
        artist="Another Artist",
        duration=240,
        file_path="/path/to/another.mp3",
        has_cover=True,
        added_at=1609459200,  # January 1, 2021 00:00:00 UTC
    )
    track2 = await repository.add(track2)
    track2_id = track2.id

    # Get all tracks (now returns Row objects)
    rows = await repository.get_all()

    # Check that both tracks are returned
    assert len(rows) == 2
    row_ids = {row.id for row in rows}
    assert track1_id in row_ids
    assert track2_id in row_ids


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_exists_by_path(repository, sample_track):
    # Add a track
    await repository.add(sample_track)

    # Check that it exists
    exists = await repository.exists_by_path(sample_track.file_path)
    assert exists is True

    # Check that a non-existent track doesn't exist
    exists = await repository.exists_by_path("/path/to/nonexistent.mp3")
    assert exists is False


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_set_cover_flag(session, sample_track):
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
@pytest.mark.filterwarnings(warning_filter)
async def test_upsert_track_insert_new(session, sample_track):
    # Create a new repository with the session
    repository = SQLiteTrackRepository(session)

    # Upsert a new track
    await repository.upsert_track(sample_track)

    # Commit the changes explicitly
    await session.commit()

    # Retrieve using the standard repository methods which should work
    rows = await repository.get_all()

    # Check that a track was inserted
    assert len(rows) == 1
    row = rows[0]
    assert row.id is not None
    assert row.title == "Test Track"
    assert row.artist == "Test Artist"
    # Note: file_path is not returned by get_all anymore


@pytest.mark.asyncio
@pytest.mark.filterwarnings(warning_filter)
async def test_upsert_track_update_existing(session, sample_track):
    # Create a new repository with the session
    repository = SQLiteTrackRepository(session)

    # First add the track
    session.add(sample_track)
    await session.commit()
    await session.refresh(sample_track)

    original_id = sample_track.id
    original_added_at = sample_track.added_at
    file_path = sample_track.file_path

    # Now update it via upsert with same file_path but different metadata
    updated_track_data = Track(
        title="Updated Title",
        artist="Updated Artist",
        duration=200,
        file_path=file_path,  # Same file path
        has_cover=True,  # Changed
        added_at=original_added_at + 100,  # Changed added_at
    )

    updated_track = await repository.upsert_track(updated_track_data)
    updated_track_id = updated_track.id
    await session.commit()

    # Retrieve the updated track using the repository method
    rows = await repository.get_all()
    assert len(rows) == 1
    retrieved_row = rows[0]

    # Check that metadata was updated
    assert updated_track_id == original_id  # Same ID
    assert retrieved_row.id == original_id  # Same ID
    # Note: added_at is not returned by get_all anymore
    # Check the updated values from the Row object
    assert retrieved_row.title == "Updated Title"  # Updated title
    assert retrieved_row.artist == "Updated Artist"  # Updated artist
    assert retrieved_row.duration == 200  # Updated duration
    assert retrieved_row.has_cover is True  # Updated has_cover
