import os
from pathlib import Path
import pyvips
from pyvips import Image as VipsImage
from typing import Any


class CoverProcessor:
    # Hardcoded covers directory path
    COVERS_DIR: str = "./data/covers"

    def __init__(self):
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
            # Load image from bytes
            image: Any = VipsImage.new_from_buffer(cover_data, "")

            # Paths for the two cover sizes
            original_path = self.covers_dir / f"{track_id}_original.webp"
            small_path = self.covers_dir / f"{track_id}_small.webp"

            # Generate original size WebP
            # Quality 85 provides a good balance between quality and file size
            # Convert path to string for pyvips
            image.webpsave(str(original_path), Q=85)

            # Create a small thumbnail (80x80)
            # Using fixed 80x80 size for the small thumbnail
            thumbnail: Any = image.thumbnail_image(
                80, height=80, crop=pyvips.Interesting.CENTRE
            )
            thumbnail.webpsave(str(small_path), Q=85)

            return True
        except Exception as e:
            # Log the error - in a production app we'd use a proper logger here
            print(f"Error processing cover for track {track_id}: {str(e)}")
            return False
