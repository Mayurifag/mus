import pytest
import asyncio
from pathlib import Path
from typing import Any
import pyvips
import shutil
from unittest.mock import patch, MagicMock, AsyncMock, ANY
import os

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


@pytest.fixture(scope="function")
def temp_covers_dir():
    test_dir_name = "./test_covers_func_ecp"
    test_dir = Path(test_dir_name)
    if test_dir.exists():
        shutil.rmtree(test_dir)
    os.makedirs(test_dir, exist_ok=True)
    yield test_dir
    shutil.rmtree(test_dir)


@pytest.fixture
def cover_processor(temp_covers_dir: Path):
    """Create a CoverProcessor instance with a temporary directory for each test."""
    processor = CoverProcessor(covers_dir_path=temp_covers_dir)
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
    async def mock_extract(_):  # Unused parameter
        await asyncio.sleep(0.01)
        return sample_image_data

    # Use AsyncMock to ensure the mock is async-aware
    with patch.object(
        cover_processor,
        "extract_cover_from_file",
        side_effect=mock_extract,
        new_callable=AsyncMock,
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

    # Use AsyncMock
    with patch.object(
        cover_processor,
        "extract_cover_from_file",
        side_effect=mock_extract,
        new_callable=AsyncMock,
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
    with patch("src.mus.infrastructure.scanner.cover_processor.MP3") as mock_mp3:
        # Mock MP3 that has tags but with corrupted APIC data
        mock_audio = MagicMock()
        mock_apic = MagicMock()
        mock_apic.data = b"corrupted_image_data_not_valid"
        mock_audio.tags = {"APIC:": mock_apic}
        mock_mp3.return_value = mock_audio

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
    with patch("src.mus.infrastructure.scanner.cover_processor.FLAC") as mock_flac:
        # Mock FLAC that has pictures but with corrupted data
        mock_audio = MagicMock()
        mock_picture = MagicMock()
        mock_picture.data = b"corrupted_flac_image_data"
        mock_audio.pictures = [mock_picture]
        mock_flac.return_value = mock_audio

        # Extract cover data
        result = await cover_processor.extract_cover_from_file(Path("corrupted.flac"))

        # Should return the corrupted data
        assert result == b"corrupted_flac_image_data"

        # But processing should fail gracefully
        process_result = await cover_processor.process_and_save_cover(101, result)
        assert process_result is False
