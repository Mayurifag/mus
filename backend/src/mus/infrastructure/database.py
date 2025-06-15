from typing import AsyncGenerator

from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker

from src.mus.config import settings

# Create engine
engine = create_async_engine(
    f"sqlite+aiosqlite:///{settings.DATABASE_PATH}",
    echo=False,  # Set to True for debugging SQL queries
)


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
