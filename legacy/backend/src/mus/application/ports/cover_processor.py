from typing import Protocol


class ICoverProcessor(Protocol):
    """Protocol defining the interface for processing and saving cover art."""

    async def process_and_save_cover(self, track_id: int, image_data: bytes) -> bool:
        """Process and save cover art in different sizes.

        Args:
            track_id: The ID of the track the cover belongs to.
            image_data: Raw bytes of the image data.

        Returns:
            bool: True if processing and saving was successful, False otherwise.
        """
        ...
