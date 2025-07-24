from fastapi import APIRouter, Depends

from src.mus.application.services.permissions_service import PermissionsService

router = APIRouter(prefix="/api/v1/system", tags=["system"])

permissions_service = PermissionsService()


def get_permissions_service() -> PermissionsService:
    return permissions_service


@router.get("/permissions")
async def get_permissions(
    service: PermissionsService = Depends(get_permissions_service),
) -> dict:
    return {"can_write_files": service.check_write_permissions()}
