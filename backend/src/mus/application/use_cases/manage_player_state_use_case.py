import logging
from src.mus.application.dtos.player_state import PlayerStateDTO
from src.mus.domain.entities.player_state import PlayerState
from src.mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)

logger = logging.getLogger(__name__)


class ManagePlayerStateUseCase:
    def __init__(self, repository: SQLitePlayerStateRepository):
        self.repository = repository

    async def save_state(self, player_state_dto: PlayerStateDTO) -> PlayerStateDTO:
        logger.info(
            f"Use case processing player state save - "
            f"Track ID: {player_state_dto.current_track_id}, "
            f"Progress: {player_state_dto.progress_seconds:.2f}s, "
            f"Volume: {player_state_dto.volume_level:.2f}, "
            f"Muted: {player_state_dto.is_muted}, "
            f"Shuffle: {player_state_dto.is_shuffle}, "
            f"Repeat: {player_state_dto.is_repeat}"
        )

        player_state = PlayerState(
            current_track_id=player_state_dto.current_track_id,
            progress_seconds=player_state_dto.progress_seconds,
            volume_level=player_state_dto.volume_level,
            is_muted=player_state_dto.is_muted,
            is_shuffle=player_state_dto.is_shuffle,
            is_repeat=player_state_dto.is_repeat,
        )

        saved_state = await self.repository.save_state(player_state)

        logger.info("Use case completed player state save successfully")

        return PlayerStateDTO.model_validate(saved_state)

    async def load_state(self) -> PlayerStateDTO:
        player_state = await self.repository.load_state()

        if player_state:
            return PlayerStateDTO.model_validate(player_state)

        return PlayerStateDTO()
