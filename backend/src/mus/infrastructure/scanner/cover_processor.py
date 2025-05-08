import os
from pathlib import Path
import pyvips
from pyvips import Image as VipsImage
from typing import Any


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
