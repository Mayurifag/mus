from mus.application.ports.player_state_repository import IPlayerStateRepository
from mus.domain.player_state import PlayerState


class LoadPlayerStateUseCase:
    def __init__(self, player_state_repository: IPlayerStateRepository):
        self._repository = player_state_repository

    async def execute(self) -> PlayerState:
        state = await self._repository.load_state()
        return state if state is not None else PlayerState.create_default()
