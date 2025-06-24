from typing import AsyncGenerator

from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
from sqlalchemy import event

from src.mus.config import settings
from src.mus.domain.entities.track import Track
from src.mus.domain.entities.track_history import TrackHistory

# Ensure SQLModel metadata includes all table definitions
__all__ = ["Track", "TrackHistory"]

# Create engine
engine = create_async_engine(
    f"sqlite+aiosqlite:///{settings.DATABASE_PATH}",
    echo=False,  # Set to True for debugging SQL queries
)


@event.listens_for(engine.sync_engine, "connect")
def set_sqlite_pragma(dbapi_connection, _connection_record):  # noqa: ARG001
    cursor = dbapi_connection.cursor()
    cursor.execute("PRAGMA journal_mode=WAL")
    cursor.execute("PRAGMA synchronous=NORMAL")
    cursor.execute("PRAGMA busy_timeout=5000")
    cursor.close()


# Use async_sessionmaker for asyncio
async_session_factory = async_sessionmaker(
    engine, class_=AsyncSession, expire_on_commit=False
)


async def get_session_generator() -> AsyncGenerator[AsyncSession, None]:
    """Generate a database session using the factory."""
    async with async_session_factory() as session:
        # The try/finally is not strictly necessary here as the async with handles exceptions
        # and ensures session.close() is called. Yielding directly is fine.
        yield session
        # No explicit commit or rollback here, that's responsibility of the caller
        # or the FastAPI dependency system that uses this generator.


async def create_db_and_tables() -> None:
    """Create database tables if they don't exist."""
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all)
