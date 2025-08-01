import os
import pytest
import pytest_asyncio
from typing import AsyncGenerator
from unittest.mock import patch
from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.ext.asyncio import create_async_engine
from sqlalchemy.pool import NullPool
import fakeredis.aioredis
import fakeredis
from src.mus.domain.entities.track import Track
from src.mus.domain.entities.player_state import PlayerState
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)
from src.mus.infrastructure.database import get_session_generator
from src.mus.main import app as fastapi_app

TEST_DB_PATH = "./test.db"
TEST_DATABASE_URL = f"sqlite+aiosqlite:///{TEST_DB_PATH}"


@pytest.fixture(scope="session", autouse=True)
def cleanup_test_db_file():
    if os.path.exists(TEST_DB_PATH):
        os.remove(TEST_DB_PATH)
    yield
    if os.path.exists(TEST_DB_PATH):
        os.remove(TEST_DB_PATH)


engine = create_async_engine(
    TEST_DATABASE_URL, poolclass=NullPool, connect_args={"check_same_thread": False}
)


@pytest_asyncio.fixture(autouse=True)
async def create_test_db():
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all)
    yield
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.drop_all)


@pytest_asyncio.fixture
async def session(create_test_db) -> AsyncGenerator[AsyncSession, None]:
    _ = create_test_db  # Fixture dependency for DB setup
    async with AsyncSession(engine) as session:
        yield session


@pytest_asyncio.fixture
async def track_repository(session):
    return SQLiteTrackRepository(session)


@pytest_asyncio.fixture
async def player_state_repository(session):
    return SQLitePlayerStateRepository(session)


@pytest_asyncio.fixture
async def sample_track():
    return Track(
        title="Test Track",
        artist="Test Artist",
        duration=180,
        file_path="/path/to/test.mp3",
        has_cover=False,
        added_at=1609459200,  # January 1, 2021 00:00:00 UTC
    )


@pytest_asyncio.fixture
async def sample_state():
    return PlayerState(
        current_track_id=42,
        progress_seconds=30,
        volume_level=0.8,
        is_muted=False,
    )


@pytest_asyncio.fixture
async def app():
    async def _get_session_override():
        async with AsyncSession(engine) as session:
            yield session

    fastapi_app.dependency_overrides[get_session_generator] = _get_session_override
    yield fastapi_app
    fastapi_app.dependency_overrides.clear()


@pytest_asyncio.fixture
async def fake_redis():
    fake_redis_instance = fakeredis.aioredis.FakeRedis()

    # Create a fake coredis instance that mimics the interface
    class FakeCoredis:
        def __init__(self):
            self._fake_redis = fake_redis_instance

        async def execute_command(self, *_args, **_kwargs):
            # Mock the execute_command method that coredis uses
            return True

        async def publish_task(self, *_args, **_kwargs):
            # Mock the publish_task method
            return True

        def __getattr__(self, name):
            # Delegate other attributes to the fake redis instance
            return getattr(self._fake_redis, name)

    fake_coredis = FakeCoredis()

    with (
        patch("redis.asyncio.from_url") as mock_async_from_url,
        patch("redis.from_url") as mock_sync_from_url,
        patch("coredis.Redis.from_url") as mock_coredis_from_url,
        patch("src.mus.core.streaq_broker.worker.redis", fake_coredis),
    ):
        mock_async_from_url.return_value = fake_redis_instance
        mock_sync_from_url.return_value = fakeredis.FakeRedis()
        mock_coredis_from_url.return_value = fake_coredis
        yield fake_redis_instance
