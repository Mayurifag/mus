from sqlalchemy import select
from sqlalchemy.dialects.sqlite import insert as sqlite_upsert
from sqlalchemy.ext.asyncio import AsyncSession

from src.mus.domain.entities.player_state import PlayerState


class SQLitePlayerStateRepository:
    def __init__(self, session: AsyncSession):
        self._session = session

    async def save_state(self, state: PlayerState) -> PlayerState:
        """Save player state using SQLite's UPSERT functionality."""
        # Always use id=1 as we only have one player state
        state_data = {
            "id": 1,
            "current_track_id": state.current_track_id,
            "progress_seconds": state.progress_seconds,
            "volume_level": state.volume_level,
            "is_muted": state.is_muted,
        }

        # Create the upsert statement
        stmt = sqlite_upsert(PlayerState).values(**state_data)

        # Add ON CONFLICT DO UPDATE clause
        stmt = stmt.on_conflict_do_update(
            index_elements=["id"],
            set_={
                key: getattr(stmt.excluded, key) for key in state_data if key != "id"
            },
        )

        await self._session.exec(stmt)
        await self._session.commit()

        # After upserting, fetch the object from the database to ensure it's the session-managed instance
        # and reflects the current state in the DB.
        # The primary key is fixed as 1.
        persisted_state = await self._session.get(PlayerState, 1)
        if persisted_state is None:
            # This case should ideally not happen if UPSERT was successful
            # and inserted the row.
            # Handling it defensively.
            raise RuntimeError(
                "Failed to retrieve player state from database after upsert."
            )
        return persisted_state

    async def load_state(self) -> PlayerState | None:
        """Load the current player state."""
        stmt = select(PlayerState).where(PlayerState.id == 1)
        result = await self._session.exec(stmt)
        return result.scalar_one_or_none()
