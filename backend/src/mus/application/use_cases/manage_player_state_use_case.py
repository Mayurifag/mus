from mus.application.dtos.player_state import PlayerStateDTO
from mus.domain.entities.player_state import PlayerState
from mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)


class ManagePlayerStateUseCase:
    def __init__(self, repository: SQLitePlayerStateRepository):
        self.repository = repository

    async def save_state(self, player_state_dto: PlayerStateDTO) -> PlayerStateDTO:
        player_state = PlayerState(
            current_track_id=player_state_dto.current_track_id,
            progress_seconds=player_state_dto.progress_seconds,
            volume_level=player_state_dto.volume_level,
            is_muted=player_state_dto.is_muted,
        )

        saved_state = await self.repository.save_state(player_state)
        return PlayerStateDTO.model_validate(saved_state)

    async def load_state(self) -> PlayerStateDTO:
        player_state = await self.repository.load_state()

        if player_state:
            return PlayerStateDTO.model_validate(player_state)

        # Return default state if none exists
        return PlayerStateDTO()
