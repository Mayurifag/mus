from typing import Any
import pytest
from pathlib import Path
import pyvips
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from unittest.mock import patch, AsyncMock


@pytest.fixture
def sample_image_data():
    """Create a simple test image."""
    # Create a simple RGB test image (50x50 pixels)
    # Use specific values for bands that will work with pyvips
    image: Any = pyvips.Image.black(50, 50, bands=3)
    # Add some color to make it identifiable
    image = image + [128, 64, 0]  # Orange-brown color
    # Convert to PNG format in memory
    buffer = image.write_to_buffer(".png")
    return buffer


# A temporary directory for covers, created once per module
@pytest.fixture(scope="module")
def module_temp_covers_dir(tmp_path_factory):
    """Create a module-scoped temporary directory for cover processing tests."""
    return tmp_path_factory.mktemp("test_covers_module_cp")


@pytest.fixture
def cover_processor(module_temp_covers_dir: Path):  # Use module-scoped temp dir
    """Create a CoverProcessor instance with a temporary directory."""
    processor = CoverProcessor(covers_dir_path=module_temp_covers_dir)
    return processor


@pytest.mark.asyncio
async def test_process_and_save_cover(cover_processor, sample_image_data):
    """Test that covers are processed and saved correctly."""
    track_id = 42

    # Process the image
    result = await cover_processor.process_and_save_cover(track_id, sample_image_data)

    # Check the result
    assert result is True

    # Check that files were created
    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    assert original_path.exists()
    assert small_path.exists()

    # For small test images like this, sometimes the small version (80x80) might be
    # larger than the original (50x50) due to compression and metadata overhead.
    # So let's just check that both files have a reasonable size.
    assert original_path.stat().st_size > 0
    assert small_path.stat().st_size > 0

    # Load the images to verify their dimensions
    original_img: Any = pyvips.Image.new_from_file(str(original_path))
    small_img: Any = pyvips.Image.new_from_file(str(small_path))

    # Check dimensions
    assert original_img.width == 50  # Should maintain original dimensions
    assert original_img.height == 50
    assert small_img.width == 80  # Should be resized to 80x80
    assert small_img.height == 80


@pytest.mark.asyncio
async def test_process_and_save_cover_error_handling(cover_processor):
    """Test error handling with invalid image data."""
    track_id = 43
    invalid_data = b"This is not valid image data"

    # Process the invalid data
    result = await cover_processor.process_and_save_cover(track_id, invalid_data)

    # Should return False when processing fails
    assert result is False

    # Files should not be created
    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    assert not original_path.exists()
    assert not small_path.exists()


@pytest.mark.asyncio
async def test_process_and_save_cover_none_data(cover_processor):
    """Test handling of None cover data."""
    track_id = 44

    result = await cover_processor.process_and_save_cover(track_id, None)

    assert result is False

    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    assert not original_path.exists()
    assert not small_path.exists()


@pytest.mark.asyncio
async def test_process_and_save_cover_empty_data(cover_processor):
    """Test handling of empty cover data."""
    track_id = 45
    empty_data = b""

    result = await cover_processor.process_and_save_cover(track_id, empty_data)

    assert result is False

    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    assert not original_path.exists()
    assert not small_path.exists()


@pytest.mark.asyncio
async def test_process_and_save_cover_corrupt_data(cover_processor):
    """Test handling of corrupt image data that might pass initial checks."""
    track_id = 46

    # Create data that looks like it might be an image but is corrupt
    corrupt_data = b"\x89PNG\r\n\x1a\n" + b"corrupt_data_here" * 10

    result = await cover_processor.process_and_save_cover(track_id, corrupt_data)

    assert result is False

    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    assert not original_path.exists()
    assert not small_path.exists()


@pytest.mark.asyncio
async def test_process_and_save_cover_with_padding(cover_processor, sample_image_data):
    """Test handling of image data with leading null bytes (common in MP3 tags)."""
    track_id = 47

    # Add null byte padding like we see in the real MP3 files
    padded_data = b"\x00" + sample_image_data

    result = await cover_processor.process_and_save_cover(track_id, padded_data)

    # Should succeed after cleaning
    assert result is True

    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    assert original_path.exists()
    assert small_path.exists()


@pytest.mark.asyncio
async def test_process_and_save_cover_only_padding(cover_processor):
    """Test handling of data that is only padding."""
    track_id = 48

    # Only null bytes and whitespace
    padding_only = b"\x00\x00 \t\n\r"

    result = await cover_processor.process_and_save_cover(track_id, padding_only)

    # Should fail since no actual image data
    assert result is False

    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    assert not original_path.exists()
    assert not small_path.exists()


@pytest.mark.asyncio
async def test_process_and_save_cover_uses_asyncio_to_thread(
    cover_processor, sample_image_data
):
    """Test that process_and_save_cover uses asyncio.to_thread to call _save_cover_sync."""
    track_id = 100

    with patch("asyncio.to_thread", new_callable=AsyncMock) as mock_to_thread:
        mock_to_thread.return_value = True

        result = await cover_processor.process_and_save_cover(
            track_id, sample_image_data
        )

        assert result is True
        mock_to_thread.assert_awaited_once()

        # Verify the correct arguments were passed to asyncio.to_thread
        call_args = mock_to_thread.call_args
        args = call_args[0]
        assert args[0] == cover_processor._save_cover_sync
        assert args[1] == track_id  # track_id
        assert args[2] == sample_image_data  # cleaned_data (no padding in this case)
        assert str(args[3]).endswith(f"{track_id}_original.webp")  # original_path
        assert str(args[4]).endswith(f"{track_id}_small.webp")  # small_path
        assert args[5] is None  # file_path


@pytest.mark.asyncio
async def test_process_and_save_cover_handles_sync_method_failure(
    cover_processor, sample_image_data
):
    """Test that process_and_save_cover handles _save_cover_sync returning False."""
    track_id = 101

    with patch("asyncio.to_thread", new_callable=AsyncMock) as mock_to_thread:
        mock_to_thread.return_value = False

        result = await cover_processor.process_and_save_cover(
            track_id, sample_image_data
        )

        assert result is False
        mock_to_thread.assert_awaited_once()


@pytest.mark.asyncio
async def test_process_and_save_cover_handles_sync_method_exception(
    cover_processor, sample_image_data
):
    """Test that process_and_save_cover handles _save_cover_sync raising an exception."""
    track_id = 102

    with patch("asyncio.to_thread", new_callable=AsyncMock) as mock_to_thread:
        mock_to_thread.side_effect = Exception("Thread execution failed")

        with pytest.raises(Exception, match="Thread execution failed"):
            await cover_processor.process_and_save_cover(track_id, sample_image_data)

        mock_to_thread.assert_awaited_once()


def test_save_cover_sync_success(cover_processor, sample_image_data):
    """Test the synchronous _save_cover_sync method directly."""
    track_id = 103
    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    result = cover_processor._save_cover_sync(
        track_id, sample_image_data, original_path, small_path
    )

    assert result is True
    assert original_path.exists()
    assert small_path.exists()


def test_save_cover_sync_failure(cover_processor):
    """Test the synchronous _save_cover_sync method with invalid data."""
    track_id = 104
    invalid_data = b"invalid image data"
    original_path = cover_processor.covers_dir / f"{track_id}_original.webp"
    small_path = cover_processor.covers_dir / f"{track_id}_small.webp"

    result = cover_processor._save_cover_sync(
        track_id, invalid_data, original_path, small_path
    )

    assert result is False
    assert not original_path.exists()
    assert not small_path.exists()
