from pathlib import Path
from typing import Protocol


class IMetadataReader(Protocol):
    async def read_metadata(
        self, file_path: Path
    ) -> tuple[str, str, int, int, bytes | None] | None:
        """Read metadata from a music file.

        Args:
            file_path: Path to the music file.

        Returns:
            tuple[str, str, int, int, Optional[bytes]] | None: A tuple containing
                (title, artist, duration, added_at, cover_data) if successful,
                None if metadata could not be read.
        """
        ...
