from mus.application.ports.player_state_repository import IPlayerStateRepository
from mus.domain.player_state import PlayerState


class SavePlayerStateUseCase:
    def __init__(self, player_state_repository: IPlayerStateRepository):
        self._repository = player_state_repository

    async def execute(self, state: PlayerState) -> None:
        await self._repository.save_state(state)
