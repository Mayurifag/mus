from contextlib import asynccontextmanager
from typing import TypeVar, Callable, Any, Optional
from src.mus.infrastructure.database import async_session_factory
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.persistence.sqlite_track_history_repository import (
    SQLiteTrackHistoryRepository,
)

T = TypeVar("T")
R = TypeVar("R")


@asynccontextmanager
async def db_operation(auto_commit: bool = True):
    """Generic database operation context manager."""
    async with async_session_factory() as session:
        try:
            yield session
            if auto_commit:
                await session.commit()
        except Exception:
            await session.rollback()
            raise


async def execute_with_track_repo(
    operation: Callable[[SQLiteTrackRepository, Any], Any], auto_commit: bool = True
) -> Any:
    """Execute an operation with track repository."""
    async with db_operation(auto_commit) as session:
        repo = SQLiteTrackRepository(session)
        return await operation(repo, session)


async def execute_with_track_history_repo(
    operation: Callable[[SQLiteTrackHistoryRepository, Any], Any],
    auto_commit: bool = True,
) -> Any:
    """Execute an operation with track history repository."""
    async with db_operation(auto_commit) as session:
        repo = SQLiteTrackHistoryRepository(session)
        return await operation(repo, session)


async def safe_db_operation(
    operation: Callable[[], Any],
    default_value: Optional[T] = None,
    log_errors: bool = True,
) -> Optional[T]:
    """Execute a database operation safely with error handling."""
    try:
        return await operation()
    except Exception as e:
        if log_errors:
            import logging

            logging.error(f"Database operation failed: {e}")
        return default_value
