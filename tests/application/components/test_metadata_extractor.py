from datetime import UTC, datetime
from pathlib import Path
from unittest.mock import patch

import pytest

from mus.application.components.metadata_extractor import MetadataExtractor


@pytest.mark.asyncio
async def test_read_metadata_success():
    extractor = MetadataExtractor()
    test_path = Path("/test/path/track.mp3")

    # Mock the file system operations
    with patch.object(Path, "stat") as mock_stat:
        mock_stat.return_value.st_mtime = 1704067200  # 2024-01-01 00:00:00 UTC
        result = await extractor.read_metadata(test_path)

    assert result is not None
    title, artist, duration, modified_at = result
    assert title == "Unknown Title"
    assert artist == "Unknown Artist"
    assert duration == 0.0
    assert isinstance(modified_at, datetime)
    assert modified_at == datetime(2024, 1, 1, tzinfo=UTC)


@pytest.mark.asyncio
async def test_read_metadata_error():
    extractor = MetadataExtractor()
    # Simulate an error by passing an invalid path
    result = await extractor.read_metadata(Path(""))

    assert result is None
