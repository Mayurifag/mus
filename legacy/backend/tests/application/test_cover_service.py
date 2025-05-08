from pathlib import Path

import pytest

from mus.application.services.cover_service import CoverService


@pytest.fixture
def cover_service(tmp_path):
    service = CoverService(covers_dir=str(tmp_path))
    return service


@pytest.fixture
def sample_image_data():
    # Create a small valid PNG image in memory
    return (
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00"
        b"\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\x0bIDAT"
        b"\x08\x99c\xf8\xff\xff?\x00\x05\xfe\x02\xfe\r\xb2\xe6\xe5\x00\x00"
        b"\x00\x00IEND\xaeB`\x82"
    )


async def test_process_and_save_cover_success(cover_service, sample_image_data):
    # Test successful cover processing
    result = await cover_service.process_and_save_cover(1, sample_image_data)
    assert result is True

    # Verify files were created
    covers_dir = Path(cover_service.covers_dir)
    assert (covers_dir / "1_small.webp").exists()
    assert (covers_dir / "1_medium.webp").exists()


async def test_process_and_save_cover_invalid_data(cover_service):
    # Test with invalid image data
    result = await cover_service.process_and_save_cover(1, b"invalid image data")
    assert result is False


async def test_process_and_save_cover_empty_data(cover_service):
    # Test with empty data
    result = await cover_service.process_and_save_cover(1, b"")
    assert result is False
