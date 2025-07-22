import logging
from pathlib import Path
from typing import List

from src.mus.config import settings
from src.mus.domain.entities.track import ProcessingStatus, Track
from src.mus.util.db_utils import upsert_tracks_batch
from src.mus.util.metadata_extractor import extract_fast_metadata

logger = logging.getLogger(__name__)

AUDIO_EXTENSIONS = {".mp3", ".flac", ".m4a", ".ogg", ".wav"}


class FastInitialScanUseCase:
    def __init__(self, music_directory: Path):
        self.music_directory = music_directory

    async def execute(self):
        logger.info("Phase 1: Fast metadata scan starting...")

        audio_files = self._find_audio_files()
        if not audio_files:
            logger.info("No audio files found.")
            return

        audio_files.sort(key=lambda f: f.stat().st_mtime, reverse=True)

        logger.info(f"Found {len(audio_files)} audio files.")

        batch_size = 100
        all_tracks = []

        for i in range(0, len(audio_files), batch_size):
            batch = audio_files[i : i + batch_size]
            metadata_results = [extract_fast_metadata(file_path) for file_path in batch]

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

        if all_tracks:
            await upsert_tracks_batch(all_tracks)
            logger.info(f"Phase 1 complete: {len(all_tracks)} tracks upserted.")

    def _find_audio_files(self) -> List[Path]:
        audio_files = []
        try:
            for file_path in self.music_directory.rglob("*"):
                if file_path.is_file() and file_path.suffix.lower() in AUDIO_EXTENSIONS:
                    audio_files.append(file_path)
        except (OSError, PermissionError) as e:
            logger.warning(f"Error scanning directory {self.music_directory}: {e}")
        return audio_files

    @classmethod
    async def create_default(cls) -> "FastInitialScanUseCase":
        music_dir = Path(settings.MUSIC_DIR_PATH)
        music_dir.mkdir(parents=True, exist_ok=True)
        return cls(music_dir)
