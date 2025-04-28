from mus.application.components.metadata_extractor import MetadataExtractor
from mus.config import DATABASE_URL, MUSIC_DIR
from mus.infrastructure.persistence.sqlite_track_repository import SQLiteTrackRepository
from mus.infrastructure.scan.file_system_scanner import FileSystemScanner


def get_track_repository() -> SQLiteTrackRepository:
    return SQLiteTrackRepository(DATABASE_URL.replace("sqlite+aiosqlite:///", ""))


def get_file_scanner() -> FileSystemScanner:
    return FileSystemScanner(MUSIC_DIR)


def get_metadata_extractor() -> MetadataExtractor:
    return MetadataExtractor()
