import logging
from fastapi import APIRouter, Depends, Request
from sqlmodel.ext.asyncio.session import AsyncSession
from src.mus.application.dtos.player_state import PlayerStateDTO
from src.mus.application.use_cases.manage_player_state_use_case import (
    ManagePlayerStateUseCase,
)
from src.mus.infrastructure.database import get_session_generator
from src.mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)

router = APIRouter(prefix="/api/v1/player", tags=["player"])
logger = logging.getLogger(__name__)


async def get_player_state_use_case(
    session: AsyncSession = Depends(get_session_generator),
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
    request: Request,
    use_case: ManagePlayerStateUseCase = Depends(get_player_state_use_case),
) -> PlayerStateDTO:
    # Determine request source (beacon vs regular API call)
    user_agent = request.headers.get("user-agent", "")
    is_beacon = (
        "beacon" in user_agent.lower()
        or request.headers.get("content-type") == "text/plain"
    )
    source = "beacon" if is_beacon else "regular API"

    logger.info(
        f"Player state save request received - Source: {source}, "
        f"Track ID: {player_state.current_track_id}, "
        f"Progress: {player_state.progress_seconds:.2f}s, "
        f"Volume: {player_state.volume_level:.2f}, "
        f"Muted: {player_state.is_muted}, "
        f"Shuffle: {player_state.is_shuffle}, "
        f"Repeat: {player_state.is_repeat}"
    )

    result = await use_case.save_state(player_state)

    logger.info(f"Player state save completed successfully - Source: {source}")

    return result
