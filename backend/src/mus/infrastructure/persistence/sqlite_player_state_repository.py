import logging
from sqlalchemy.dialects.sqlite import insert as sqlite_upsert
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.player_state import PlayerState

logger = logging.getLogger(__name__)


class SQLitePlayerStateRepository:
    def __init__(self, session: AsyncSession):
        self._session = session

    async def save_state(self, state: PlayerState) -> PlayerState:
        logger.info(
            f"Repository persisting player state to database - "
            f"Track ID: {state.current_track_id}, "
            f"Progress: {state.progress_seconds:.2f}s, "
            f"Volume: {state.volume_level:.2f}, "
            f"Muted: {state.is_muted}, "
            f"Shuffle: {state.is_shuffle}, "
            f"Repeat: {state.is_repeat}"
        )

        state_data = {
            "id": 1,
            "current_track_id": state.current_track_id,
            "progress_seconds": state.progress_seconds,
            "volume_level": state.volume_level,
            "is_muted": state.is_muted,
            "is_shuffle": state.is_shuffle,
            "is_repeat": state.is_repeat,
        }

        stmt = sqlite_upsert(PlayerState).values(**state_data)
        stmt = stmt.on_conflict_do_update(
            index_elements=["id"],
            set_={
                "current_track_id": stmt.excluded.current_track_id,
                "progress_seconds": stmt.excluded.progress_seconds,
                "volume_level": stmt.excluded.volume_level,
                "is_muted": stmt.excluded.is_muted,
                "is_shuffle": stmt.excluded.is_shuffle,
                "is_repeat": stmt.excluded.is_repeat,
            },
        )
        await self._session.execute(stmt)
        await self._session.commit()

        persisted_state = await self._session.get(PlayerState, 1)
        if persisted_state is None:
            raise RuntimeError(
                "Failed to retrieve player state from database after upsert."
            )

        return persisted_state

    async def load_state(self) -> PlayerState | None:
        stmt = select(PlayerState).where(PlayerState.id == 1)
        result = await self._session.exec(stmt)
        return result.one_or_none()
