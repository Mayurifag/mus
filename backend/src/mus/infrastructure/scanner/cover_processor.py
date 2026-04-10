import os
import asyncio
from pathlib import Path
from typing import Any, Optional
from mutagen._file import File as MutagenFile
import logging

logger = logging.getLogger(__name__)

_pyvips_initialized = False


def _get_pyvips():
    import pyvips

    global _pyvips_initialized
    if not _pyvips_initialized:
        pyvips.cache_set_max(0)
        pyvips.cache_set_max_mem(0)
        pyvips.cache_set_max_files(0)
        pyvips.logger.setLevel(pyvips.logging.ERROR)
        _pyvips_initialized = True
    return pyvips


class CoverProcessor:
    def __init__(self, covers_dir_path: Path) -> None:
        self.covers_dir = covers_dir_path
        os.makedirs(self.covers_dir, exist_ok=True)

    async def process_and_save_cover(
        self,
        track_id: int,
        cover_data: Optional[bytes],
        file_path: Optional[Path] = None,
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
            pyvips = _get_pyvips()
            image: Any = pyvips.Image.new_from_buffer(cover_data_cleaned, "")
            image.webpsave(str(original_path), Q=95, strip=True)
            thumbnail: Any = image.thumbnail_image(
                80, height=80, crop=pyvips.Interesting.CENTRE
            )
            thumbnail.webpsave(str(small_path), Q=85, strip=True)
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
        try:
            audio = MutagenFile(file_path, easy=False)
            if not audio:
                return None

            # For FLAC files and others with embedded pictures
            if hasattr(audio, "pictures") and audio.pictures:
                return audio.pictures[0].data

            # For files with ID3 tags (MP3, WAV, etc.) or MP4 tags (M4A)
            if audio.tags:
                # ID3v2 APIC frames
                for key in audio.tags:
                    if key.startswith("APIC:"):
                        return audio.tags[key].data
                # MP4 'covr' atom
                if "covr" in audio.tags and audio.tags["covr"]:
                    return audio.tags["covr"][0]

            return None
        except Exception as e:
            logger.warning(f"Error during sync cover extraction for {file_path}: {e}")
            return None
