from pathlib import Path

import structlog

from mus.application.ports.cover_processor import ICoverProcessor
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
        cover_processor: ICoverProcessor,
    ):
        self._file_scanner = file_scanner
        self._metadata_reader = metadata_reader
        self._track_repository = track_repository
        self._cover_processor = cover_processor

    async def execute(self, directory_path: Path) -> None:
        logger.info("starting_track_scan", directory=str(directory_path))

        async for file_path in self._file_scanner.find_music_files(directory_path):
            if await self._track_repository.exists_by_path(file_path):
                logger.info("track_already_exists", file_path=str(file_path))
                continue

            metadata = await self._metadata_reader.read_metadata(file_path)
            if metadata is None:
                continue

            title, artist, duration, added_at, cover_data = metadata
            track = Track(
                title=title,
                artist=artist,
                duration=duration,
                file_path=file_path,
                added_at=added_at,
            )

            track_id = await self._track_repository.add(track)
            if track_id is None:
                logger.error("failed_to_add_track", file_path=str(file_path))
                continue

            # Process cover art if available
            if cover_data is not None:
                if await self._cover_processor.process_and_save_cover(
                    track_id, cover_data
                ):
                    await self._track_repository.set_cover_flag(track_id, True)
                    logger.info("cover_art_processed", track_id=track_id)
                else:
                    logger.warning("cover_art_processing_failed", track_id=track_id)

            logger.info(
                "track_added",
                file_path=str(file_path),
                title=title,
                artist=artist,
                track_id=track_id,
                has_cover=track.has_cover,
            )

        logger.info("track_scan_completed", directory=str(directory_path))
