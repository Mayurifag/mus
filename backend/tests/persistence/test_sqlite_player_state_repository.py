import pytest
import pytest_asyncio
from sqlmodel import SQLModel
from sqlalchemy.ext.asyncio import create_async_engine
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.player_state import PlayerState
from src.mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
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
    return SQLitePlayerStateRepository(session)


@pytest_asyncio.fixture
async def sample_state():
    return PlayerState(
        current_track_id=42,
        progress_seconds=30,
        volume_level=0.8,
        is_muted=False,
    )


@pytest.mark.asyncio
async def test_save_and_load_state(setup_test_db, repository, sample_state):
    """Test saving and loading player state."""
    # Save the state
    saved_state = await repository.save_state(sample_state)

    # Check that it was saved with ID=1
    assert saved_state.id == 1
    assert saved_state.current_track_id == 42
    assert saved_state.progress_seconds == 30
    assert saved_state.volume_level == 0.8
    assert saved_state.is_muted is False

    # Load the state and verify
    loaded_state = await repository.load_state()
    assert loaded_state is not None
    assert loaded_state.id == 1
    assert loaded_state.current_track_id == 42
    assert loaded_state.progress_seconds == 30
    assert loaded_state.volume_level == 0.8
    assert loaded_state.is_muted is False


@pytest.mark.asyncio
async def test_update_existing_state(setup_test_db, repository, sample_state):
    """Test updating an existing player state."""
    # First save the initial state
    await repository.save_state(sample_state)

    # Now update it with new values
    updated_state = PlayerState(
        current_track_id=43,  # Changed
        progress_seconds=60,  # Changed
        volume_level=0.5,  # Changed
        is_muted=True,  # Changed
    )

    saved_state = await repository.save_state(updated_state)

    # Check that the state was updated
    assert saved_state.id == 1  # Same ID
    assert saved_state.current_track_id == 43
    assert saved_state.progress_seconds == 60
    assert saved_state.volume_level == 0.5
    assert saved_state.is_muted is True

    # Verify by loading
    loaded_state = await repository.load_state()
    assert loaded_state is not None
    assert loaded_state.id == 1
    assert loaded_state.current_track_id == 43
    assert loaded_state.progress_seconds == 60
    assert loaded_state.volume_level == 0.5
    assert loaded_state.is_muted is True


@pytest.mark.asyncio
async def test_load_nonexistent_state(setup_test_db, repository):
    """Test loading a state that doesn't exist yet."""
    state = await repository.load_state()
    assert state is None


@pytest.mark.asyncio
async def test_upsert_state_insert_new(setup_test_db, repository, sample_state):
    """Test upserting a new player state."""
    # Upsert a new state
    state = await repository.upsert_state(sample_state)

    # Check that it was inserted with ID=1
    assert state.id == 1
    assert state.current_track_id == 42
    assert state.progress_seconds == 30
    assert state.volume_level == 0.8
    assert state.is_muted is False

    # Verify with a separate query
    loaded_state = await repository.load_state()
    assert loaded_state is not None
    assert loaded_state.id == 1
    assert loaded_state.current_track_id == 42


@pytest.mark.asyncio
async def test_upsert_state_update_existing(setup_test_db, repository, sample_state):
    """Test upserting a player state that already exists."""
    # First add the state
    original_state = await repository.save_state(sample_state)
    assert original_state.id == 1  # Should have ID=1

    # Now update it via upsert with different values
    updated_values = {
        "current_track_id": 43,  # Changed
        "progress_seconds": 60,  # Changed
        "volume_level": 0.5,  # Changed
        "is_muted": True,  # Changed
    }

    # Create a new state object without an ID to test the upsert functionality
    updated_state = PlayerState(**updated_values)

    # The repository should handle setting the ID to 1
    upserted_state = await repository.upsert_state(updated_state)

    # Check that values were updated
    assert upserted_state.id == 1  # Always 1 for player state
    assert upserted_state.current_track_id == 43
    assert upserted_state.progress_seconds == 60
    assert upserted_state.volume_level == 0.5
    assert upserted_state.is_muted is True

    # Verify only one state exists
    loaded_state = await repository.load_state()
    assert loaded_state is not None
    assert loaded_state.id == 1
    assert loaded_state.current_track_id == 43
