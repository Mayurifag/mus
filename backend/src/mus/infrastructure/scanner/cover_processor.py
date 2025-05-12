import os
import asyncio
from pathlib import Path
import pyvips
from pyvips import Image as VipsImage
from typing import Any, Optional, Dict, List, Tuple
from mutagen.flac import FLAC
from mutagen.id3 import ID3
from mutagen.mp3 import MP3


class CoverProcessor:
    COVERS_DIR: str = "./data/covers"

    def __init__(self) -> None:
        # Create covers directory if it doesn't exist
        os.makedirs(self.COVERS_DIR, exist_ok=True)
        self.covers_dir = Path(self.COVERS_DIR)

        # Suppress excessive logging from pyvips, only show errors
        if hasattr(pyvips, "logger") and hasattr(pyvips.logger, "setLevel"):
            pyvips.logger.setLevel(pyvips.logging.ERROR)

    async def process_and_save_cover(self, track_id: int, cover_data: bytes) -> bool:
        """
        Process and save cover art for a track

        Generates two versions:
        - original: WebP at original dimensions
        - small: WebP thumbnail (80x80)

        Returns True if successful, False otherwise
        """
        try:
            image: Any = VipsImage.new_from_buffer(cover_data, "")

            original_path = self.covers_dir / f"{track_id}_original.webp"
            small_path = self.covers_dir / f"{track_id}_small.webp"

            image.webpsave(str(original_path), Q=100)

            # Using fixed 80x80 size for the small thumbnail
            thumbnail: Any = image.thumbnail_image(
                80, height=80, crop=pyvips.Interesting.CENTRE
            )
            thumbnail.webpsave(str(small_path), Q=90)

            return True
        except Exception:
            return False

    async def extract_cover_from_file(self, file_path: Path) -> Optional[bytes]:
        """
        Extract cover art from an audio file using mutagen

        Supports MP3 (ID3) and FLAC formats

        Returns the raw cover art data as bytes if found, None otherwise
        """
        # Use anyio.to_thread.run_sync to run mutagen (which is CPU bound) in a separate thread
        return await asyncio.to_thread(self._extract_cover_sync, file_path)

    def _extract_cover_sync(self, file_path: Path) -> Optional[bytes]:
        """Synchronous implementation of cover extraction to be run in a thread"""
        try:
            file_ext = file_path.suffix.lower()

            if file_ext == ".mp3":
                return self._extract_mp3_cover(file_path)
            elif file_ext == ".flac":
                return self._extract_flac_cover(file_path)
            else:
                return None
        except Exception:
            # If any extraction errors occur, we just return None
            return None

    def _extract_mp3_cover(self, file_path: Path) -> Optional[bytes]:
        """Extract cover art from MP3 files using ID3 tags"""
        try:
            audio = MP3(file_path, ID3=ID3)

            # No ID3 tags
            if audio.tags is None:
                return None

            # Look for APIC frames (album art)
            for key in audio.tags.keys():
                if key.startswith("APIC:"):
                    return audio.tags[key].data

            return None
        except Exception:
            return None

    def _extract_flac_cover(self, file_path: Path) -> Optional[bytes]:
        """Extract cover art from FLAC files"""
        try:
            audio = FLAC(file_path)

            # FLAC files store pictures in the pictures attribute
            if audio.pictures:
                # Get the first picture (typically cover art)
                return audio.pictures[0].data

            return None
        except Exception:
            return None

    async def process_tracks_covers_batch(
        self, tracks_data: List[Tuple[int, Path]]
    ) -> Dict[int, bool]:
        """
        Process covers for a batch of tracks concurrently

        Args:
            tracks_data: List of tuples containing (track_id, file_path)

        Returns:
            Dict mapping track_id to success status (True if cover was processed)
        """
        results: Dict[int, bool] = {}

        # Process in batches of 10 to avoid overwhelming the system
        batch_size = 10
        for i in range(0, len(tracks_data), batch_size):
            batch = tracks_data[i : i + batch_size]

            # Create tasks for extracting and processing covers
            tasks = []
            for track_id, file_path in batch:
                task = asyncio.create_task(
                    self._process_single_track_cover(track_id, file_path)
                )
                tasks.append((track_id, task))

            # Wait for all tasks to complete
            for track_id, task in tasks:
                try:
                    results[track_id] = await task
                except Exception:
                    results[track_id] = False

        return results

    async def _process_single_track_cover(self, track_id: int, file_path: Path) -> bool:
        """Process a single track's cover art"""
        cover_data = await self.extract_cover_from_file(file_path)
        if cover_data:
            return await self.process_and_save_cover(track_id, cover_data)
        return False
