from typing import Optional
from fastapi import Depends

from sqlmodel import select
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.domain.entities.player_state import PlayerState
from src.mus.infrastructure.database import get_session_generator


class SQLitePlayerStateRepository:
    def __init__(self, session: AsyncSession = Depends(get_session_generator)):
        self.session = session

    async def save_state(self, state: PlayerState) -> PlayerState:
        existing_state = await self.load_state()

        if existing_state:
            # Update existing state
            existing_state.current_track_id = state.current_track_id
            existing_state.progress_seconds = state.progress_seconds
            existing_state.volume_level = state.volume_level
            existing_state.is_muted = state.is_muted

            self.session.add(existing_state)
            await self.session.commit()
            await self.session.refresh(existing_state)
            return existing_state
        else:
            # Create new state
            new_state = PlayerState(
                id=1,  # Always use id=1 for singleton player state
                current_track_id=state.current_track_id,
                progress_seconds=state.progress_seconds,
                volume_level=state.volume_level,
                is_muted=state.is_muted,
            )
            self.session.add(new_state)
            await self.session.commit()
            await self.session.refresh(new_state)
            return new_state

    async def load_state(self) -> Optional[PlayerState]:
        statement = select(PlayerState).where(PlayerState.id == 1)
        result = await self.session.exec(statement)
        return result.first()

    async def upsert_state(self, state: PlayerState) -> PlayerState:
        """
        Update or insert player state using SQLite's UPSERT (ON CONFLICT DO UPDATE).

        Always uses id=1 as the primary key for the singleton player state.
        """
        # First check if a record already exists
        existing_state = await self.load_state()

        if existing_state:
            # Update existing state
            existing_state.current_track_id = state.current_track_id
            existing_state.progress_seconds = state.progress_seconds
            existing_state.volume_level = state.volume_level
            existing_state.is_muted = state.is_muted

            self.session.add(existing_state)
            await self.session.commit()
            await self.session.refresh(existing_state)
            return existing_state
        else:
            # Insert a new state
            state_to_save = PlayerState(
                id=1,
                current_track_id=state.current_track_id,
                progress_seconds=state.progress_seconds,
                volume_level=state.volume_level,
                is_muted=state.is_muted,
            )

            # Insert the state
            self.session.add(state_to_save)
            await self.session.commit()
            await self.session.refresh(state_to_save)
            return state_to_save
