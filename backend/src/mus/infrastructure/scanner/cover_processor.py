import os
import asyncio
from pathlib import Path
import pyvips
from pyvips import Image as VipsImage
from typing import Any, Optional, Dict, List, Tuple
from mutagen.flac import FLAC
from mutagen.id3 import ID3
from mutagen.mp3 import MP3
from mutagen.wave import WAVE
import logging

logger = logging.getLogger(__name__)


class CoverProcessor:
    def __init__(self, covers_dir_path: Path) -> None:
        self.covers_dir = covers_dir_path
        os.makedirs(self.covers_dir, exist_ok=True)

        # Suppress excessive logging from pyvips, only show errors
        if hasattr(pyvips, "logger") and hasattr(pyvips.logger, "setLevel"):
            pyvips.logger.setLevel(pyvips.logging.ERROR)

    async def process_and_save_cover(
        self, track_id: int, cover_data: bytes, file_path: Optional[Path] = None
    ) -> bool:
        if not cover_data:
            return False

        # Strip leading null bytes and whitespace
        cleaned_data = cover_data.lstrip(b"\x00 \t\n\r")
        if not cleaned_data:
            return False

        original_path = self.covers_dir / f"{track_id}_original.webp"
        small_path = self.covers_dir / f"{track_id}_small.webp"

        return await asyncio.to_thread(
            self._save_cover_sync,
            track_id,
            cleaned_data,
            original_path,
            small_path,
            file_path,
        )

    def _save_cover_sync(
        self,
        track_id: int,
        cover_data_cleaned: bytes,
        original_path: Path,
        small_path: Path,
        file_path: Optional[Path] = None,
    ) -> bool:
        try:
            image: Any = VipsImage.new_from_buffer(cover_data_cleaned, "")
            image.webpsave(str(original_path), Q=90)
            # Using fixed 80x80 size for the small thumbnail
            thumbnail: Any = image.thumbnail_image(
                80, height=80, crop=pyvips.Interesting.CENTRE
            )
            thumbnail.webpsave(str(small_path), Q=85)
            return True
        except Exception as e:
            error_msg = f"Error processing cover for track {track_id}"
            if file_path:
                error_msg += f" (file: {file_path})"
            error_msg += f": {e}"
            logger.error(error_msg)
            return False

    async def extract_cover_from_file(self, file_path: Path) -> Optional[bytes]:
        return await asyncio.to_thread(self._extract_cover_sync, file_path)

    def _extract_cover_sync(self, file_path: Path) -> Optional[bytes]:
        # TODO: pattern matching refactoring
        try:
            file_ext = file_path.suffix.lower()
            if file_ext == ".mp3":
                return self._extract_mp3_cover(file_path)
            elif file_ext == ".flac":
                return self._extract_flac_cover(file_path)
            elif file_ext == ".wav":
                return self._extract_wav_cover(file_path)
            return None
        except Exception as e:
            logger.warning(f"Error during sync cover extraction for {file_path}: {e}")
            return None

    def _extract_mp3_cover(self, file_path: Path) -> Optional[bytes]:
        try:
            audio = MP3(file_path, ID3=ID3)
            if audio.tags is None:
                return None

            for key in audio.tags.keys():
                if key.startswith("APIC:"):
                    return audio.tags[key].data
            return None
        except Exception as e:
            logger.warning(f"Failed to extract MP3 cover from {file_path}: {e}")
            return None

    def _extract_flac_cover(self, file_path: Path) -> Optional[bytes]:
        try:
            audio = FLAC(file_path)

            if audio.pictures:
                return audio.pictures[0].data
            return None
        except Exception as e:
            logger.warning(f"Failed to extract FLAC cover from {file_path}: {e}")
            return None

    def _extract_wav_cover(self, file_path: Path) -> Optional[bytes]:
        try:
            audio = WAVE(file_path)

            if hasattr(audio, "tags") and audio.tags:
                for key in audio.tags.keys():
                    if key.startswith("APIC:"):
                        return audio.tags[key].data
            return None
        except Exception as e:
            logger.warning(f"Failed to extract WAV cover from {file_path}: {e}")
            return None

    # TODO: make this optimal
    async def process_tracks_covers_batch(
        self, tracks_data: List[Tuple[int, Path]]
    ) -> Dict[int, bool]:
        """
        Process covers for a batch of tracks concurrently.
        Args: tracks_data: List of (track_id, file_path).
        Returns: Dict mapping track_id to success status.
        """
        results: Dict[int, bool] = {}
        # Process in batches of 10 to avoid overwhelming the system
        batch_size = 10
        for i in range(0, len(tracks_data), batch_size):
            batch = tracks_data[i : i + batch_size]
            tasks_with_ids = [
                (
                    track_id,
                    asyncio.create_task(
                        self._process_single_track_cover(track_id, file_path)
                    ),
                )
                for track_id, file_path in batch
            ]
            for track_id, task in tasks_with_ids:
                try:
                    results[track_id] = await task
                except Exception as e:
                    logger.error(
                        f"Error processing cover task for track_id {track_id}: {e}"
                    )
                    results[track_id] = False
        return results

    async def _process_single_track_cover(self, track_id: int, file_path: Path) -> bool:
        """Process a single track's cover art."""
        cover_data = await self.extract_cover_from_file(file_path)
        if cover_data:
            return await self.process_and_save_cover(track_id, cover_data, file_path)
        return False
