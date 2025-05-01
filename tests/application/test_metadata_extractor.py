import pytest
from mutagen.flac import FLAC, Picture
from mutagen.id3 import ID3
from mutagen.id3._frames import APIC

from mus.application.components.metadata_extractor import MetadataExtractor


@pytest.fixture
def metadata_extractor():
    return MetadataExtractor()


@pytest.fixture
def sample_mp3_with_cover(tmp_path):
    # Create a temporary MP3 file with embedded cover art
    file_path = tmp_path / "test.mp3"

    # Create a minimal valid MP3 file (1 second of silence)
    with open(file_path, "wb") as f:
        # MP3 header (MPEG 1 Layer 3, 44.1kHz, 128kbps)
        header = bytes(
            [
                0xFF,
                0xFB,  # MPEG Audio sync word
                0x90,  # MPEG-1 Layer 3, 44.1kHz
                0x64,  # 128kbps, stereo
            ]
        )
        # Write 38 frames (1 second of audio)
        for _ in range(38):
            f.write(header)
            # Each frame is 417 bytes at 128kbps
            f.write(b"\x00" * 413)  # Frame data

    # Add ID3 tags with cover art
    tags = ID3()
    tags.add(
        APIC(
            encoding=3, mime="image/jpeg", type=3, desc="Cover", data=b"fake_jpeg_data"
        )
    )
    tags.save(file_path)
    return file_path


@pytest.fixture
def sample_flac_with_cover(tmp_path):
    # Create a temporary FLAC file with embedded cover art
    file_path = tmp_path / "test.flac"

    # Create a minimal valid FLAC file (1 second of silence)
    with open(file_path, "wb") as f:
        # FLAC stream marker
        f.write(b"fLaC")

        # STREAMINFO metadata block (mandatory)
        f.write(
            bytes(
                [
                    0x80,  # Last metadata block + STREAMINFO
                    0x00,
                    0x00,
                    0x22,  # Length of STREAMINFO (34 bytes)
                ]
            )
        )

        # STREAMINFO content
        streaminfo = bytes(
            [
                0x00,
                0x00,  # Min block size (0)
                0x00,
                0x00,  # Max block size (0)
                0x00,
                0x00,
                0x00,  # Min frame size
                0x00,
                0x00,
                0x00,  # Max frame size
                0x44,
                0xAC,
                0x00,
                0x00,  # Sample rate (44.1kHz)
                0x10,  # Channels (2) + Bits per sample (16)
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,  # Total samples
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,  # MD5 signature
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
            ]
        )
        f.write(streaminfo)

        # Write a minimal audio frame
        f.write(
            bytes(
                [
                    0xFF,  # Frame header marker
                    0xF8,  # Frame header
                    0x00,  # Frame header
                    0x00,  # Frame header
                ]
            )
        )
        f.write(b"\x00" * 16)  # Minimal frame content

    # Add picture metadata
    flac = FLAC(file_path)
    picture = Picture()
    picture.type = 3  # Cover (front)
    picture.mime = "image/png"
    picture.desc = "Cover"
    picture.width = 100
    picture.height = 100
    picture.depth = 24
    picture.data = b"fake_png_data"
    flac.add_picture(picture)
    flac.save()

    return file_path


async def test_extract_cover_from_mp3(metadata_extractor, sample_mp3_with_cover):
    metadata = await metadata_extractor.read_metadata(sample_mp3_with_cover)
    assert metadata is not None
    _, _, _, _, cover_data = metadata
    assert cover_data == b"fake_jpeg_data"


async def test_extract_cover_from_flac(metadata_extractor, sample_flac_with_cover):
    metadata = await metadata_extractor.read_metadata(sample_flac_with_cover)
    assert metadata is not None
    _, _, _, _, cover_data = metadata
    assert cover_data == b"fake_png_data"


async def test_no_cover_art(metadata_extractor, tmp_path):
    # Create a minimal valid MP3 file without cover art
    file_path = tmp_path / "no_cover.mp3"

    # Create a minimal valid MP3 file (1 second of silence)
    with open(file_path, "wb") as f:
        # MP3 header (MPEG 1 Layer 3, 44.1kHz, 128kbps)
        header = bytes(
            [
                0xFF,
                0xFB,  # MPEG Audio sync word
                0x90,  # MPEG-1 Layer 3, 44.1kHz
                0x64,  # 128kbps, stereo
            ]
        )
        # Write 38 frames (1 second of audio)
        for _ in range(38):
            f.write(header)
            # Each frame is 417 bytes at 128kbps
            f.write(b"\x00" * 413)  # Frame data

    # Add empty ID3 tags
    tags = ID3()
    tags.save(file_path)

    metadata = await metadata_extractor.read_metadata(file_path)
    assert metadata is not None
    _, _, _, _, cover_data = metadata
    assert cover_data is None
