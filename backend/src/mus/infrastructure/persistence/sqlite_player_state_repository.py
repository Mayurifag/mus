from sqlalchemy.dialects.sqlite import insert as sqlite_upsert
from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.player_state import PlayerState


class SQLitePlayerStateRepository:
    def __init__(self, session: AsyncSession):
        self._session = session

    async def save_state(self, state: PlayerState) -> PlayerState:
        state_data = {
            "id": 1,
            "current_track_id": state.current_track_id,
            "progress_seconds": state.progress_seconds,
            "volume_level": state.volume_level,
            "is_muted": state.is_muted,
        }

        stmt = sqlite_upsert(PlayerState).values(**state_data)
        stmt = stmt.on_conflict_do_update(
            index_elements=["id"],
            set_={
                key: getattr(stmt.excluded, key)
                for key, value in state_data.items()
                if key != "id"
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
