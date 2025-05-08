import os
from typing import AsyncGenerator

from sqlmodel import SQLModel
from sqlmodel.ext.asyncio.session import AsyncSession
from sqlalchemy.ext.asyncio import create_async_engine
from sqlalchemy.pool import NullPool

from mus.config import settings

# Ensure data directory exists for SQLite file
if settings.DATABASE_URL.startswith("sqlite") and ":///" in settings.DATABASE_URL:
    db_path = settings.DATABASE_URL.split("///")[1]
    os.makedirs(os.path.dirname(db_path), exist_ok=True)

# Ensure we use aiosqlite for async operations with SQLite
if settings.DATABASE_URL.startswith("sqlite"):
    engine = create_async_engine(
        settings.DATABASE_URL,
        poolclass=NullPool,
        connect_args={"check_same_thread": False},
    )
else:
    engine = create_async_engine(settings.DATABASE_URL, poolclass=NullPool)


async def get_session() -> AsyncGenerator[AsyncSession, None]:
    async with AsyncSession(engine) as session:
        yield session


async def init_db() -> None:
    async with engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all)
