from fastapi import APIRouter, Depends

from mus.application.use_cases.save_player_state import SavePlayerStateUseCase
from mus.dependencies import get_save_player_state_use_case
from mus.domain.player_state import PlayerState

router = APIRouter(prefix="/state", tags=["state"])


def get_save_player_state_use_case_dependency() -> SavePlayerStateUseCase:
    return get_save_player_state_use_case()


save_player_state_dependency = Depends(get_save_player_state_use_case_dependency)


@router.post("")
async def save_state(
    state: PlayerState,
    save_player_state: SavePlayerStateUseCase = save_player_state_dependency,
) -> None:
    await save_player_state.execute(state)


save_state = router.post("")(save_state)
