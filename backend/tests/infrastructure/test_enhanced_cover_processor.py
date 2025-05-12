import pytest
import asyncio
from pathlib import Path
from typing import Any
import pyvips
import tempfile
import shutil
from unittest.mock import patch, MagicMock, ANY

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
def test_covers_dir():
    """Create a temporary directory for test covers."""
    test_dir = tempfile.mkdtemp(prefix="test_covers_")
    yield Path(test_dir)
    # Cleanup
    shutil.rmtree(test_dir, ignore_errors=True)


@pytest.fixture
def cover_processor(test_covers_dir):
    """Create a CoverProcessor instance with a temporary directory."""
    processor = CoverProcessor()
    processor.COVERS_DIR = str(test_covers_dir)
    processor.covers_dir = test_covers_dir
    return processor


@pytest.mark.asyncio
async def test_extract_cover_from_file_mp3(cover_processor):
    """Test the MP3 cover extraction."""
    # Mock MP3 file extraction
    with patch("src.mus.infrastructure.scanner.cover_processor.MP3") as mock_mp3:
        # Setup the mock
        mock_audio = MagicMock()
        mock_audio.tags = {"APIC:": MagicMock(data=b"test_cover_data")}
        mock_mp3.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.mp3"))

        # Verify result
        assert result == b"test_cover_data"
        mock_mp3.assert_called_once_with(Path("test.mp3"), ID3=ANY)


@pytest.mark.asyncio
async def test_extract_cover_from_file_flac(cover_processor):
    """Test the FLAC cover extraction."""
    # Mock FLAC file extraction
    with patch("src.mus.infrastructure.scanner.cover_processor.FLAC") as mock_flac:
        # Setup the mock
        mock_audio = MagicMock()
        mock_picture = MagicMock(data=b"flac_cover_data")
        mock_audio.pictures = [mock_picture]
        mock_flac.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.flac"))

        # Verify result
        assert result == b"flac_cover_data"
        mock_flac.assert_called_once_with(Path("test.flac"))


@pytest.mark.asyncio
async def test_extract_cover_from_file_no_cover_mp3(cover_processor):
    """Test MP3 extraction when no cover art is present."""
    # Mock MP3 file extraction with no tags
    with patch("src.mus.infrastructure.scanner.cover_processor.MP3") as mock_mp3:
        # Setup the mock
        mock_audio = MagicMock()
        mock_audio.tags = {}  # Empty tags
        mock_mp3.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.mp3"))

        # Verify result is None when no cover found
        assert result is None


@pytest.mark.asyncio
async def test_extract_cover_from_file_no_cover_flac(cover_processor):
    """Test FLAC extraction when no cover art is present."""
    # Mock FLAC file extraction with no pictures
    with patch("src.mus.infrastructure.scanner.cover_processor.FLAC") as mock_flac:
        # Setup the mock
        mock_audio = MagicMock()
        mock_audio.pictures = []  # No pictures
        mock_flac.return_value = mock_audio

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.flac"))

        # Verify result is None when no cover found
        assert result is None


@pytest.mark.asyncio
async def test_extract_cover_from_file_unsupported_format(cover_processor):
    """Test extraction with an unsupported file format."""
    result = await cover_processor.extract_cover_from_file(Path("test.wav"))
    assert result is None


@pytest.mark.asyncio
async def test_extract_cover_from_file_exception(cover_processor):
    """Test error handling during cover extraction."""
    # Mock MP3 file extraction that raises an exception
    with patch("src.mus.infrastructure.scanner.cover_processor.MP3") as mock_mp3:
        # Setup the mock to raise an exception
        mock_mp3.side_effect = Exception("Test exception")

        # Test extraction
        result = await cover_processor.extract_cover_from_file(Path("test.mp3"))

        # Should gracefully handle exception and return None
        assert result is None


@pytest.mark.asyncio
async def test_process_tracks_covers_batch(cover_processor, sample_image_data):
    """Test batch processing of track covers."""

    # Mock extract_cover_from_file to return test image data
    async def mock_extract(file_path):
        await asyncio.sleep(0.01)
        return sample_image_data

    with patch.object(
        cover_processor,
        "extract_cover_from_file",
        side_effect=mock_extract,
    ):
        # Create test data: [(track_id, file_path), ...]
        test_tracks = [
            (1, Path("test1.mp3")),
            (2, Path("test2.flac")),
            (3, Path("test3.mp3")),
        ]

        # Process batch
        results = await cover_processor.process_tracks_covers_batch(test_tracks)

        # Verify results
        assert len(results) == 3
        assert all(results.values())  # All should be True
        assert results[1] is True
        assert results[2] is True
        assert results[3] is True

        # Verify files were created
        for track_id in [1, 2, 3]:
            original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
            small_path = cover_processor.covers_dir / f"{track_id}_small.webp"
            assert original_path.exists()
            assert small_path.exists()


@pytest.mark.asyncio
async def test_process_tracks_covers_batch_with_error(
    cover_processor, sample_image_data
):
    """Test batch processing with an error during extraction."""

    # Mock extract_cover_from_file to return data for some tracks and None for others
    async def mock_extract(file_path):
        await asyncio.sleep(0.01)  # Small delay to simulate processing
        if "test2" in str(file_path):
            return None  # Simulate extraction failure for test2
        return sample_image_data

    with patch.object(
        cover_processor, "extract_cover_from_file", side_effect=mock_extract
    ):
        # Create test data
        test_tracks = [
            (1, Path("test1.mp3")),
            (2, Path("test2.flac")),  # This one will fail extraction
            (3, Path("test3.mp3")),
        ]

        # Process batch
        results = await cover_processor.process_tracks_covers_batch(test_tracks)

        # Verify results
        assert len(results) == 3
        assert results[1] is True  # Success
        assert results[2] is False  # Failed extraction
        assert results[3] is True  # Success

        # Verify files were created for successful extractions only
        assert (cover_processor.covers_dir / "1_original.webp").exists()
        assert (cover_processor.covers_dir / "1_small.webp").exists()
        assert not (cover_processor.covers_dir / "2_original.webp").exists()
        assert not (cover_processor.covers_dir / "2_small.webp").exists()
        assert (cover_processor.covers_dir / "3_original.webp").exists()
        assert (cover_processor.covers_dir / "3_small.webp").exists()


@pytest.mark.asyncio
async def test_process_single_track_cover(cover_processor, sample_image_data):
    """Test processing a single track's cover."""
    # Mock extract_cover_from_file to return sample image data
    with patch.object(
        cover_processor, "extract_cover_from_file", return_value=sample_image_data
    ):
        # Process single track
        result = await cover_processor._process_single_track_cover(42, Path("test.mp3"))

        # Verify result
        assert result is True

        # Verify files were created
        original_path = cover_processor.covers_dir / "42_original.webp"
        small_path = cover_processor.covers_dir / "42_small.webp"
        assert original_path.exists()
        assert small_path.exists()


@pytest.mark.asyncio
async def test_process_single_track_cover_extraction_failure(cover_processor):
    """Test single track processing when extraction fails."""
    # Mock extract_cover_from_file to return None (extraction failure)
    with patch.object(cover_processor, "extract_cover_from_file", return_value=None):
        # Process single track
        result = await cover_processor._process_single_track_cover(42, Path("test.mp3"))

        # Verify result
        assert result is False

        # Verify no files were created
        original_path = cover_processor.covers_dir / "42_original.webp"
        small_path = cover_processor.covers_dir / "42_small.webp"
        assert not original_path.exists()
        assert not small_path.exists()
