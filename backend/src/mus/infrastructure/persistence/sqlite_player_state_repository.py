from sqlalchemy import Column, Integer, select
from sqlalchemy.dialects.sqlite import insert as sqlite_upsert
from sqlalchemy.ext.asyncio import AsyncSession

from mus.domain.entities.player_state import PlayerState


class SQLitePlayerStateRepository:
    def __init__(self, session: AsyncSession):
        self._session = session

    async def save_state(self, state: PlayerState) -> PlayerState:
        """Save player state using SQLite's UPSERT functionality."""
        # Always use id=1 as we only have one player state
        state.id = 1

        # Create the upsert statement
        stmt = sqlite_upsert(PlayerState).values(
            id=state.id,
            current_track_id=state.current_track_id,
            progress_seconds=state.progress_seconds,
            volume_level=state.volume_level,
            is_muted=state.is_muted,
        )

        # Add ON CONFLICT DO UPDATE clause
        stmt = stmt.on_conflict_do_update(
            index_elements=["id"],  # Use string column name instead of model class
            set_=dict(
                current_track_id=stmt.excluded.current_track_id,
                progress_seconds=stmt.excluded.progress_seconds,
                volume_level=stmt.excluded.volume_level,
                is_muted=stmt.excluded.is_muted,
            ),
        )

        # Execute the statement and return the state
        await self._session.execute(stmt)
        await self._session.commit()
        return state

    async def load_state(self) -> PlayerState | None:
        """Load the current player state."""
        stmt = select(PlayerState).where(
            Column("id", Integer) == 1
        )  # Use SQLAlchemy Column
        result = await self._session.execute(stmt)
        return result.scalar_one_or_none()
