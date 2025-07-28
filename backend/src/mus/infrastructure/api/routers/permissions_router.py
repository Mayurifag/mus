from fastapi import APIRouter, Depends

from src.mus.application.services.permissions_service import PermissionsService
from src.mus.infrastructure.api.dependencies import get_permissions_service

router = APIRouter(prefix="/api/v1/system", tags=["system"])


@router.get("/permissions")
async def get_permissions(
    service: PermissionsService = Depends(get_permissions_service),
) -> dict:
    return {"can_write_music_files": service.can_write_music_files}
