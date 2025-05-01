import asyncio
import os
from pathlib import Path

import structlog

from mus.application.use_cases.scan_tracks import ScanTracksUseCase
from mus.domain.repositories.track_repository import ITrackRepository

logger = structlog.get_logger()


class FullScanInteractor:
    """Performs a full system scan, including database reset and cover cache
    clearing."""

    def __init__(
        self,
        track_repository: ITrackRepository,
        scan_tracks_use_case: ScanTracksUseCase,
        db_path: str,
        covers_dir: Path,
        music_dir: Path,
    ) -> None:
        """Initialize the FullScanInteractor.

        Args:
            track_repository: Repository for track operations
            scan_tracks_use_case: Use case for scanning tracks
            db_path: Path to the SQLite database file
            covers_dir: Directory containing cover art cache
            music_dir: Directory containing music files
        """
        self._track_repository = track_repository
        self._scan_tracks_use_case = scan_tracks_use_case
        self._db_path = db_path
        self._covers_dir = covers_dir
        self._music_dir = music_dir

    async def execute(self) -> None:
        """Execute the full scan process.

        This method:
        1. Removes the existing database file
        2. Reinitializes the database schema
        3. Clears the cover art cache
        4. Triggers a full music scan
        """
        try:
            # Remove existing database
            try:
                os.remove(self._db_path)
                logger.info("Removed existing database file", path=self._db_path)
            except FileNotFoundError:
                logger.info("No existing database file to remove", path=self._db_path)

            # Initialize fresh database schema
            await self._track_repository.initialize_schema()
            logger.info("Initialized database schema")

            # Clear cover cache
            await self._clear_cover_cache()

            # Perform full scan
            await self._scan_tracks_use_case.execute(self._music_dir)
            logger.info("Completed full scan")

        except Exception as e:
            logger.exception("Failed to perform full scan", exc_info=e)
            raise

    async def _clear_cover_cache(self) -> None:
        """Clear all cached cover art files."""
        if not self._covers_dir.exists():
            logger.info("Cover cache directory does not exist", path=self._covers_dir)
            return

        try:
            # Use asyncio.to_thread for file operations to avoid blocking
            def delete_covers() -> None:
                for file_path in self._covers_dir.glob("*.webp"):
                    try:
                        file_path.unlink()
                    except OSError as e:
                        logger.warning(
                            "Failed to delete cover file",
                            path=file_path,
                            error=str(e),
                        )

            await asyncio.to_thread(delete_covers)
            logger.info("Cleared cover cache", path=self._covers_dir)

        except Exception as e:
            logger.warning("Error while clearing cover cache", exc_info=e)
            # Don't raise the exception as this is not critical for the scan process
