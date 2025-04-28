from datetime import UTC, datetime
from pathlib import Path

import structlog

from mus.application.ports.metadata_reader import IMetadataReader

logger = structlog.get_logger()


class MetadataExtractor(IMetadataReader):
    async def read_metadata(
        self, file_path: Path
    ) -> tuple[str, str, float, datetime] | None:
        try:
            if not file_path.name:  # Check if path is empty
                raise ValueError("Empty file path")

            # TODO: Implement actual metadata extraction in Phase 4
            # For now, return dummy data with file modification time
            return (
                "Unknown Title",
                "Unknown Artist",
                0.0,
                datetime.fromtimestamp(file_path.stat().st_mtime, tz=UTC),
            )
        except Exception as e:
            logger.warning(
                "failed_to_read_metadata", file_path=str(file_path), error=str(e)
            )
            return None
