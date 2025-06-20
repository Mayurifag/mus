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

    track2 = Track(
        title="Another Track",
        artist="Another Artist",
        duration=240,
        file_path="/path/to/another.mp3",
        has_cover=True,
        added_at=1609459200,  # January 1, 2021 00:00:00 UTC
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

    # Upsert a new track and get its file_path for later reference
    await repository.upsert_track(sample_track)
    file_path = sample_track.file_path

    # Commit the changes explicitly
    await session.commit()

    # Retrieve using the standard repository methods which should work
    tracks = await repository.get_all()

    # Check that a track was inserted
    assert len(tracks) == 1
    track = tracks[0]
    assert track.id is not None
    assert track.title == "Test Track"
    assert track.artist == "Test Artist"
    assert track.file_path == file_path


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
    await session.commit()

    # Retrieve the updated track using the repository method
    tracks = await repository.get_all()
    assert len(tracks) == 1
    retrieved_track = tracks[0]

    # Check that metadata was updated but added_at was preserved
    assert updated_track.id == original_id  # Same ID
    assert retrieved_track.id == original_id  # Same ID
    assert (
        updated_track.added_at == original_added_at
    )  # Preserved original added_at timestamp
    assert (
        retrieved_track.added_at == original_added_at
    )  # Preserved original added_at timestamp
    assert updated_track.title == "Updated Title"  # Updated title
    assert updated_track.artist == "Updated Artist"  # Updated artist
    assert updated_track.duration == 200  # Updated duration
    assert updated_track.has_cover is True  # Updated has_cover
