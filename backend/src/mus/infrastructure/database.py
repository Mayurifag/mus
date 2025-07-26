from typing import AsyncGenerator

from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
from sqlalchemy import event

from src.mus.config import settings
from src.mus.domain.entities.track import Track

# Ensure SQLModel metadata includes all table definitions
__all__ = ["Track"]


# This function will be called for each new connection.
# The @event.listens_for decorator is removed to allow for explicit listener attachment.
def set_sqlite_pragma(dbapi_connection, _connection_record):  # noqa: ARG001
    cursor = dbapi_connection.cursor()
    cursor.execute("PRAGMA journal_mode=WAL")
    cursor.execute("PRAGMA synchronous=NORMAL")
    cursor.execute("PRAGMA busy_timeout=5000")
    cursor.close()


# Create engine for the main process
engine = create_async_engine(
    f"sqlite+aiosqlite:///{settings.DATABASE_PATH}",
    echo=False,  # Set to True for debugging SQL queries
)
# Explicitly attach the listener to the engine
event.listen(engine.sync_engine, "connect", set_sqlite_pragma)


# Use async_sessionmaker for asyncio
async_session_factory = async_sessionmaker(
    engine, class_=AsyncSession, expire_on_commit=False
)


def recreate_engine_after_fork():
    """Re-create the database engine and session factory after a process fork."""
    global engine, async_session_factory

    engine = create_async_engine(
        f"sqlite+aiosqlite:///{settings.DATABASE_PATH}",
        echo=False,
    )
    # Re-apply the listener to the new engine for the child process
    event.listen(engine.sync_engine, "connect", set_sqlite_pragma)

    async_session_factory = async_sessionmaker(
        engine, class_=AsyncSession, expire_on_commit=False
    )


async def get_session_generator() -> AsyncGenerator[AsyncSession, None]:
    """Generate a database session using the factory."""
    async with async_session_factory() as session:
        yield session


async def create_db_and_tables() -> None:
    """Create database tables if they don't exist."""
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all)
