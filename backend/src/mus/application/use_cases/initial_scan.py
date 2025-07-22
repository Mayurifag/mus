import asyncio
import logging
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from typing import List

from src.mus.application.use_cases.process_track_metadata import (
    process_slow_metadata_for_track,
)
from src.mus.config import settings
from src.mus.domain.entities.track import ProcessingStatus, Track
from src.mus.util.db_utils import get_tracks_by_status, upsert_tracks_batch
from src.mus.util.metadata_extractor import extract_fast_metadata

logger = logging.getLogger(__name__)

AUDIO_EXTENSIONS = {".mp3", ".flac", ".m4a", ".ogg", ".wav"}


class InitialScanUseCase:
    """Orchestrates the two-phase initial library scan."""

    def __init__(self, music_directory: Path):
        self.music_directory = music_directory

    async def execute(self):
        """Execute the two-phase initial scan."""
        logger.info("Starting initial library scan...")

        # Phase 1: Fast parallel metadata extraction
        await self._phase_1_fast_scan()

        # Phase 2: Slow metadata processing (direct, not via queue)
        await self._phase_2_slow_processing()

        logger.info("Initial library scan completed")

    async def _phase_1_fast_scan(self):
        """Phase 1: Parallel fast metadata extraction with bulk upsert."""
        logger.info("Phase 1: Fast metadata scan starting...")

        # Find all audio files
        audio_files = self._find_audio_files()
        if not audio_files:
            logger.info("No audio files found")
            return

        # Sort by modification time descending (newest first)
        audio_files.sort(key=lambda f: f.stat().st_mtime, reverse=True)

        logger.info(f"Found {len(audio_files)} audio files")

        # Process files in parallel batches
        batch_size = 100
        all_tracks = []

        for i in range(0, len(audio_files), batch_size):
            batch = audio_files[i : i + batch_size]

            # Use ProcessPoolExecutor for CPU-bound metadata extraction
            with ProcessPoolExecutor() as executor:
                loop = asyncio.get_event_loop()
                tasks = [
                    loop.run_in_executor(executor, extract_fast_metadata, file_path)
                    for file_path in batch
                ]
                metadata_results = await asyncio.gather(*tasks)

            # Convert to Track objects
            batch_tracks = []
            for file_path, metadata in zip(batch, metadata_results):
                if metadata:
                    track = Track(
                        title=metadata["title"],
                        artist=metadata["artist"],
                        duration=metadata["duration"],
                        file_path=str(file_path),
                        added_at=metadata["added_at"],
                        updated_at=metadata["added_at"],
                        inode=metadata["inode"],
                        processing_status=ProcessingStatus.PENDING,
                    )
                    batch_tracks.append(track)

            all_tracks.extend(batch_tracks)
            logger.info(
                f"Processed batch {i // batch_size + 1}/{(len(audio_files) + batch_size - 1) // batch_size}"
            )

        # Bulk upsert all tracks
        if all_tracks:
            await upsert_tracks_batch(all_tracks)
            logger.info(f"Phase 1 complete: {len(all_tracks)} tracks upserted")

    async def _phase_2_slow_processing(self):
        """Phase 2: Direct slow metadata processing (not via queue)."""
        logger.info("Phase 2: Slow metadata processing starting...")

        # Get all pending tracks
        pending_tracks = await get_tracks_by_status(ProcessingStatus.PENDING)
        if not pending_tracks:
            logger.info("No pending tracks to process")
            return

        logger.info(f"Processing {len(pending_tracks)} pending tracks...")

        # Process in batches to avoid overwhelming the system
        batch_size = 10
        processed_count = 0

        for i in range(0, len(pending_tracks), batch_size):
            batch = pending_tracks[i : i + batch_size]

            # Process batch concurrently
            tasks = [
                process_slow_metadata_for_track(track.id)
                for track in batch
                if track.id is not None
            ]

            await asyncio.gather(*tasks, return_exceptions=True)
            processed_count += len(batch)

            logger.info(f"Processed {processed_count}/{len(pending_tracks)} tracks")

        logger.info("Phase 2 complete: All pending tracks processed")

    def _find_audio_files(self) -> List[Path]:
        """Recursively find all audio files in the music directory."""
        audio_files = []

        try:
            for file_path in self.music_directory.rglob("*"):
                if file_path.is_file() and file_path.suffix.lower() in AUDIO_EXTENSIONS:
                    audio_files.append(file_path)
        except (OSError, PermissionError) as e:
            logger.warning(f"Error scanning directory {self.music_directory}: {e}")

        return audio_files

    @classmethod
    async def create_default(cls) -> "InitialScanUseCase":
        """Create an InitialScanUseCase with default music directory."""
        music_dir = Path(settings.MUSIC_DIR_PATH)
        music_dir.mkdir(parents=True, exist_ok=True)
        return cls(music_dir)
