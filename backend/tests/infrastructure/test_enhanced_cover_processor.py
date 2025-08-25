import pytest
from pathlib import Path
from typing import Any
import pyvips
from unittest.mock import patch, MagicMock

from src.mus.infrastructure.scanner.cover_processor import CoverProcessor


@pytest.fixture
def sample_image_data():
    """Create a simple test image."""
    # Create a simple RGB test image (50x50 pixels)
    image: Any = pyvips.Image.black(50, 50, bands=3)
    # Add some color to make it identifiable
    image = image + [128, 64, 0]  # Orange-brown color
    # Convert to PNG format in memory
    buffer = image.write_to_buffer(".png")
    return buffer


@pytest.fixture
def cover_processor(tmp_path: Path):
    """Create a CoverProcessor instance with a temporary directory for each test."""
    processor = CoverProcessor(covers_dir_path=tmp_path)
    return processor


@pytest.mark.asyncio
async def test_extract_cover_from_file_mp3(cover_processor):
    """Test the MP3 cover extraction."""
    # Mock MutagenFile for MP3 extraction
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        # Setup the mock - MP3 files don't have pictures attribute
        mock_audio = MagicMock()
        mock_audio.tags = {"APIC:": MagicMock(data=b"test_cover_data")}
        # Ensure pictures attribute doesn't exist or is empty
        mock_audio.pictures = []
        mock_mutagen.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.mp3"))

        # Verify result
        assert result == b"test_cover_data"
        mock_mutagen.assert_called_once_with(Path("test.mp3"), easy=False)


@pytest.mark.asyncio
async def test_extract_cover_from_file_flac(cover_processor):
    """Test the FLAC cover extraction."""
    # Mock MutagenFile for FLAC extraction
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        # Setup the mock - FLAC files use pictures
        mock_audio = MagicMock()
        mock_picture = MagicMock(data=b"flac_cover_data")
        mock_audio.pictures = [mock_picture]
        mock_audio.tags = None  # FLAC doesn't need tags for pictures
        mock_mutagen.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.flac"))

        # Verify result
        assert result == b"flac_cover_data"
        mock_mutagen.assert_called_once_with(Path("test.flac"), easy=False)


@pytest.mark.asyncio
async def test_extract_cover_from_file_no_cover_mp3(cover_processor):
    """Test MP3 extraction when no cover art is present."""
    # Mock MutagenFile for MP3 extraction with no tags
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        # Setup the mock - MP3 with no cover
        mock_audio = MagicMock()
        mock_audio.tags = {}  # Empty tags
        mock_audio.pictures = []  # No pictures
        mock_mutagen.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.mp3"))

        # Verify result is None when no cover found
        assert result is None


@pytest.mark.asyncio
async def test_extract_cover_from_file_no_cover_flac(cover_processor):
    """Test FLAC extraction when no cover art is present."""
    # Mock MutagenFile for FLAC extraction with no pictures
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        # Setup the mock - FLAC with no pictures
        mock_audio = MagicMock()
        mock_audio.pictures = []  # No pictures
        mock_audio.tags = None  # No tags either
        mock_mutagen.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.flac"))

        # Verify result is None when no cover found
        assert result is None


@pytest.mark.asyncio
async def test_extract_cover_from_file_unsupported_format(cover_processor):
    """Test extraction with an unsupported file format."""
    # Mock MutagenFile returning None for unsupported format
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        mock_mutagen.return_value = None
        result = await cover_processor.extract_cover_from_file(Path("test.wav"))
        assert result is None


@pytest.mark.asyncio
async def test_extract_cover_from_file_exception(cover_processor):
    """Test error handling during cover extraction."""
    # Mock MutagenFile that raises an exception
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        # Setup the mock to raise an exception
        mock_mutagen.side_effect = Exception("Test exception")

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.mp3"))

        # Should gracefully handle exception and return None
        assert result is None


@pytest.mark.asyncio
async def test_process_and_save_cover_with_invalid_buffer(cover_processor):
    """Test process_and_save_cover with various invalid buffer scenarios."""
    track_id = 999

    # Test with None
    result = await cover_processor.process_and_save_cover(track_id, None)
    assert result is False

    # Test with empty bytes
    result = await cover_processor.process_and_save_cover(track_id, b"")
    assert result is False

    # Test with invalid image data
    result = await cover_processor.process_and_save_cover(
        track_id, b"invalid_image_data"
    )
    assert result is False

    # Verify no files were created for any of these
    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"
    assert not original_path.exists()
    assert not small_path.exists()


@pytest.mark.asyncio
async def test_extract_cover_with_corrupted_mp3_tags(cover_processor):
    """Test extraction from MP3 with corrupted tag data."""
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        # Mock MutagenFile that has tags but with corrupted APIC data
        mock_audio = MagicMock()
        mock_apic = MagicMock()
        mock_apic.data = b"corrupted_image_data_not_valid"
        mock_audio.tags = {"APIC:": mock_apic}
        mock_audio.pictures = []  # No pictures for MP3
        mock_mutagen.return_value = mock_audio

        # Extract cover data
        result = await cover_processor.extract_cover_from_file(Path("corrupted.mp3"))

        # Should return the corrupted data (extraction doesn't validate)
        assert result == b"corrupted_image_data_not_valid"

        # But processing should fail gracefully
        process_result = await cover_processor.process_and_save_cover(100, result)
        assert process_result is False


@pytest.mark.asyncio
async def test_extract_cover_with_corrupted_flac_tags(cover_processor):
    """Test extraction from FLAC with corrupted picture data."""
    with patch(
        "src.mus.infrastructure.scanner.cover_processor.MutagenFile"
    ) as mock_mutagen:
        # Mock MutagenFile that has pictures but with corrupted data
        mock_audio = MagicMock()
        mock_picture = MagicMock()
        mock_picture.data = b"corrupted_flac_image_data"
        mock_audio.pictures = [mock_picture]
        mock_audio.tags = None  # FLAC uses pictures, not tags
        mock_mutagen.return_value = mock_audio

        # Extract cover data
        result = await cover_processor.extract_cover_from_file(Path("corrupted.flac"))

        # Should return the corrupted data
        assert result == b"corrupted_flac_image_data"

        # But processing should fail gracefully
        process_result = await cover_processor.process_and_save_cover(101, result)
        assert process_result is False
