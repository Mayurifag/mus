from pathlib import Path

import structlog

from mus.application.ports.file_scanner import IFileScanner
from mus.application.ports.metadata_reader import IMetadataReader
from mus.application.ports.track_repository import ITrackRepository
from mus.domain.track import Track

logger = structlog.get_logger()


class ScanTracksUseCase:
    def __init__(
        self,
        file_scanner: IFileScanner,
        metadata_reader: IMetadataReader,
        track_repository: ITrackRepository,
    ):
        self._file_scanner = file_scanner
        self._metadata_reader = metadata_reader
        self._track_repository = track_repository

    async def execute(self, directory_path: Path) -> None:
        logger.info("starting_track_scan", directory=str(directory_path))

        file_paths = await self._file_scanner.find_music_files(directory_path)
        async for file_path in file_paths:
            if await self._track_repository.exists_by_path(file_path):
                logger.info("track_already_exists", file_path=str(file_path))
                continue

            metadata = await self._metadata_reader.read_metadata(file_path)
            if metadata is None:
                continue

            title, artist, duration, added_at = metadata
            track = Track(
                title=title,
                artist=artist,
                duration=duration,
                file_path=file_path,
                added_at=added_at,
            )

            await self._track_repository.add(track)
            logger.info("track_added", file_path=str(file_path))

        logger.info("track_scan_completed", directory=str(directory_path))
