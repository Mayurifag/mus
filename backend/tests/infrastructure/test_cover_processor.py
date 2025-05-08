import os
from typing import Any
import pytest
from pathlib import Path
import pyvips
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor


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


@pytest.fixture
def cover_processor():
    """Create a CoverProcessor instance with a temporary directory."""
    # Use a temporary directory for testing
    test_covers_dir = "./test_covers"
    os.makedirs(test_covers_dir, exist_ok=True)

    # Create processor with the test directory
    processor = CoverProcessor()
    processor.COVERS_DIR = test_covers_dir
    processor.covers_dir = Path(test_covers_dir)

    yield processor

    # Cleanup: remove test files after test
    for file in os.listdir(test_covers_dir):
        if file.endswith(".webp"):
            os.remove(os.path.join(test_covers_dir, file))
    os.rmdir(test_covers_dir)


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
