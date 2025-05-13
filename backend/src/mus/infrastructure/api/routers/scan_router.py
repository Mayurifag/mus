from fastapi import APIRouter, Depends
from sqlmodel.ext.asyncio.session import AsyncSession

from src.mus.application.dtos.scan import ScanRequestDTO, ScanResponseDTO
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.infrastructure.database import get_session_generator
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor

router = APIRouter(prefix="/api/v1", tags=["scan"])


async def get_scan_tracks_use_case(
    session: AsyncSession = Depends(get_session_generator),
) -> ScanTracksUseCase:
    track_repository = SQLiteTrackRepository(session)
    file_system_scanner = FileSystemScanner()
    cover_processor = CoverProcessor()
    return ScanTracksUseCase(track_repository, file_system_scanner, cover_processor)


@router.post("/scan", response_model=ScanResponseDTO)
async def scan_music_directory(
    scan_request: ScanRequestDTO = ScanRequestDTO(),
    use_case: ScanTracksUseCase = Depends(get_scan_tracks_use_case),
) -> ScanResponseDTO:
    try:
        return await use_case.scan_directory(scan_request.directory_paths)
    except Exception as e:
        # Return a structured error response with status code 500
        return ScanResponseDTO(
            success=False,
            message=f"Error during scan: {str(e)}",
            tracks_added=0,
            tracks_updated=0,
            errors=1,
            error_details=[str(e)],
        )
