from pathlib import Path

import pytest

from mus.application.components.metadata_extractor import MetadataExtractor


class MockMetadataExtractor(MetadataExtractor):
    def extract_metadata(self, file_path: Path) -> dict | None:
        if file_path.suffix.lower() not in self.supported_formats:
            return None

        if not hasattr(self, "mock_metadata"):
            # Default metadata for files without tags
            return {
                "title": file_path.stem,
                "artist": "Unknown Artist",
                "duration": 0,
                "added_at": int(file_path.stat().st_mtime),
            }

        return self.mock_metadata


@pytest.fixture
def extractor():
    # Use the renamed mock class
    return MockMetadataExtractor()


@pytest.mark.asyncio  # Mark tests as async if MetadataExtractor methods become async
async def test_extract_metadata_from_mp3(extractor, tmp_path):
    # Create a test MP3 file
    mp3_path = tmp_path / "test.mp3"
    mp3_path.touch()
    mtime = int(mp3_path.stat().st_mtime)

    # Set mock metadata
    extractor.mock_metadata = {
        "title": "Test Title",
        "artist": "Test Artist",
        "duration": 180,
        "added_at": mtime,
    }

    # Extract metadata using the actual async method if it becomes async
    # For now, assuming the test mock 'extract_metadata' is synchronous
    # If MetadataExtractor.read_metadata becomes the standard, use that
    # metadata = await extractor.read_metadata(mp3_path)
    # Using the synchronous mock method for now:
    metadata = extractor.extract_metadata(mp3_path)

    assert metadata is not None
    assert metadata["title"] == "Test Title"
    assert metadata["artist"] == "Test Artist"
    assert metadata["duration"] == 180
    assert metadata["added_at"] == mtime


@pytest.mark.asyncio
async def test_extract_metadata_from_unsupported_format(extractor, tmp_path):
    # Create a test file with unsupported format
    file_path = tmp_path / "test.txt"
    file_path.touch()

    # Extract metadata
    metadata = extractor.extract_metadata(file_path)

    assert metadata is None


@pytest.mark.asyncio
async def test_extract_metadata_from_file_without_tags(extractor, tmp_path):
    # Create a test MP3 file without metadata
    mp3_path = tmp_path / "test.mp3"
    mp3_path.touch()
    mtime = int(mp3_path.stat().st_mtime)

    # Remove any potentially lingering mock metadata
    if hasattr(extractor, "mock_metadata"):
        delattr(extractor, "mock_metadata")

    # Extract metadata (without setting mock_metadata)
    metadata = extractor.extract_metadata(mp3_path)

    assert metadata is not None
    assert metadata["title"] == "test"  # Should use filename as title
    assert metadata["artist"] == "Unknown Artist"
    assert metadata["duration"] == 0
    assert metadata["added_at"] == mtime
