from fastapi import APIRouter, Depends
from sqlmodel.ext.asyncio.session import AsyncSession

from mus.application.dtos.player_state import PlayerStateDTO
from mus.application.use_cases.manage_player_state_use_case import (
    ManagePlayerStateUseCase,
)
from mus.infrastructure.database import get_session
from mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)

router = APIRouter(prefix="/api/v1/player", tags=["player"])


async def get_player_state_use_case(
    session: AsyncSession = Depends(get_session),
) -> ManagePlayerStateUseCase:
    repository = SQLitePlayerStateRepository(session)
    return ManagePlayerStateUseCase(repository)


@router.get("/state", response_model=PlayerStateDTO)
async def get_player_state(
    use_case: ManagePlayerStateUseCase = Depends(get_player_state_use_case),
) -> PlayerStateDTO:
    return await use_case.load_state()


@router.post("/state", response_model=PlayerStateDTO)
async def save_player_state(
    player_state: PlayerStateDTO,
    use_case: ManagePlayerStateUseCase = Depends(get_player_state_use_case),
) -> PlayerStateDTO:
    return await use_case.save_state(player_state)
