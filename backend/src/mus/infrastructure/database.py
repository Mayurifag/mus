from typing import AsyncGenerator

from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.ext.asyncio import create_async_engine


# Hardcoded database file path
DATABASE_FILE_PATH = "mus_database.db"

# Create engine
engine = create_async_engine(
    f"sqlite+aiosqlite:///{DATABASE_FILE_PATH}",
    echo=False,  # Set to True for debugging SQL queries
)


async def get_session_generator() -> AsyncGenerator[AsyncSession, None]:
    """Generate a database session."""
    session = AsyncSession(engine)
    try:
        yield session
    finally:
        await session.close()


async def create_db_and_tables() -> None:
    """Create database tables if they don't exist."""
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all)
