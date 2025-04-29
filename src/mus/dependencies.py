from mus.application.components.metadata_extractor import MetadataExtractor
from mus.application.use_cases.scan_tracks import ScanTracksUseCase
from mus.application.use_cases.search_tracks import SearchTracksUseCase
from mus.config import get_database_url, get_music_dir
from mus.infrastructure.persistence.sqlite_track_repository import SQLiteTrackRepository
from mus.infrastructure.scan.file_system_scanner import FileSystemScanner


def get_track_repository() -> SQLiteTrackRepository:
    return SQLiteTrackRepository(get_database_url().replace("sqlite+aiosqlite:///", ""))


def get_file_scanner() -> FileSystemScanner:
    return FileSystemScanner(get_music_dir())


def get_metadata_extractor() -> MetadataExtractor:
    return MetadataExtractor()


def get_scan_tracks_use_case() -> ScanTracksUseCase:
    return ScanTracksUseCase(
        track_repository=get_track_repository(),
        file_scanner=get_file_scanner(),
        metadata_reader=get_metadata_extractor(),
    )


def get_search_tracks_use_case() -> SearchTracksUseCase:
    return SearchTracksUseCase(track_repository=get_track_repository())
