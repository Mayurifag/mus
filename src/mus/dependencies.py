from mus.application.components.metadata_extractor import MetadataExtractor
from mus.application.services.cover_service import CoverService
from mus.application.use_cases.load_player_state import LoadPlayerStateUseCase
from mus.application.use_cases.save_player_state import SavePlayerStateUseCase
from mus.application.use_cases.scan_tracks import ScanTracksUseCase
from mus.application.use_cases.search_tracks import SearchTracksUseCase
from mus.config import get_database_url, get_music_dir
from mus.infrastructure.persistence.sqlite_player_state_repository import (
    SQLitePlayerStateRepository,
)
from mus.infrastructure.persistence.sqlite_track_repository import SQLiteTrackRepository
from mus.infrastructure.scan.file_system_scanner import FileSystemScanner
from mus.infrastructure.web.services.initial_state_service import InitialStateService


def get_track_repository() -> SQLiteTrackRepository:
    return SQLiteTrackRepository(get_database_url().replace("sqlite+aiosqlite:///", ""))


def get_file_scanner() -> FileSystemScanner:
    return FileSystemScanner(get_music_dir())


def get_metadata_extractor() -> MetadataExtractor:
    return MetadataExtractor()


def get_cover_service() -> CoverService:
    return CoverService()


def get_scan_tracks_use_case() -> ScanTracksUseCase:
    return ScanTracksUseCase(
        track_repository=get_track_repository(),
        file_scanner=get_file_scanner(),
        metadata_reader=get_metadata_extractor(),
        cover_processor=get_cover_service(),
    )


def get_search_tracks_use_case() -> SearchTracksUseCase:
    return SearchTracksUseCase(track_repository=get_track_repository())


def get_player_state_repository() -> SQLitePlayerStateRepository:
    return SQLitePlayerStateRepository(
        get_database_url().replace("sqlite+aiosqlite:///", "")
    )


def get_load_player_state_use_case() -> LoadPlayerStateUseCase:
    return LoadPlayerStateUseCase(player_state_repository=get_player_state_repository())


def get_save_player_state_use_case() -> SavePlayerStateUseCase:
    return SavePlayerStateUseCase(player_state_repository=get_player_state_repository())


def get_initial_state_service() -> InitialStateService:
    return InitialStateService(
        load_player_state_use_case=get_load_player_state_use_case(),
        track_repository=get_track_repository(),
    )
