import asyncio
import os
from typing import Final

import pyvips

from ..ports.cover_processor import ICoverProcessor


class CoverService(ICoverProcessor):
    """Service for processing and saving cover art images."""

    SMALL_SIZE: Final[tuple[int, int]] = (80, 80)
    MEDIUM_SIZE: Final[tuple[int, int]] = (128, 128)
    DEFAULT_COVERS_DIR: Final[str] = "/app/data/covers"

    def __init__(self, covers_dir: str | None = None) -> None:
        """Initialize the cover service and ensure the covers directory exists.

        Args:
            covers_dir: Optional custom path for storing covers. If not provided,
                      uses the default path.
        """
        self.covers_dir = covers_dir or self.DEFAULT_COVERS_DIR
        os.makedirs(self.covers_dir, exist_ok=True)

    async def process_and_save_cover(self, track_id: int, image_data: bytes) -> bool:
        """Process and save cover art in different sizes.

        Args:
            track_id: The ID of the track the cover belongs to.
            image_data: Raw bytes of the image data.

        Returns:
            bool: True if processing and saving was successful, False otherwise.
        """
        try:
            return await asyncio.to_thread(self._process_cover, track_id, image_data)
        except Exception:
            return False

    def _process_cover(self, track_id: int, image_data: bytes) -> bool:
        """Process cover art synchronously in a separate thread.

        Args:
            track_id: The ID of the track the cover belongs to.
            image_data: Raw bytes of the image data.

        Returns:
            bool: True if processing and saving was successful, False otherwise.
        """
        try:
            # Load image from buffer
            image = pyvips.Image.new_from_buffer(image_data, "")

            # Process small size
            small = image.thumbnail_image(
                self.SMALL_SIZE[0], height=self.SMALL_SIZE[1], size=pyvips.Size.FORCE
            )
            small_path = os.path.join(self.covers_dir, f"{track_id}_small.webp")
            small.webpsave(small_path, Q=100, lossless=True, strip=True)

            # Process medium size
            medium = image.thumbnail_image(
                self.MEDIUM_SIZE[0], height=self.MEDIUM_SIZE[1], size=pyvips.Size.FORCE
            )
            medium_path = os.path.join(self.covers_dir, f"{track_id}_medium.webp")
            medium.webpsave(medium_path, Q=100, lossless=True, strip=True)

            return True

        except pyvips.Error:
            return False
