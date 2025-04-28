import pytest

from mus.infrastructure.scan.file_system_scanner import FileSystemScanner


@pytest.fixture
def scanner(tmp_path):
    return FileSystemScanner(tmp_path)


@pytest.mark.asyncio
async def test_find_music_files(scanner, tmp_path):
    # Create test files
    (tmp_path / "test.mp3").touch()
    (tmp_path / "test.flac").touch()
    (tmp_path / "test.txt").touch()
    (tmp_path / "subdir").mkdir()
    (tmp_path / "subdir" / "test.ogg").touch()
    (tmp_path / "subdir" / "test.m4a").touch()

    # Collect all music files
    files = [f async for f in scanner.find_music_files()]

    # Check that we found all music files and no non-music files
    assert len(files) == 4  # mp3, flac, ogg, m4a
    extensions = {f.suffix.lower() for f in files}
    assert extensions == {".mp3", ".flac", ".ogg", ".m4a"}

    # Check that we found files in subdirectories
    assert any("subdir" in str(f) for f in files)


@pytest.mark.asyncio
async def test_skip_inaccessible_directories(scanner, tmp_path):
    # Create a directory we can't access
    inaccessible_dir = tmp_path / "inaccessible"
    inaccessible_dir.mkdir()
    (inaccessible_dir / "test.mp3").touch()
    inaccessible_dir.chmod(0o000)  # Make directory inaccessible

    try:
        # Collect all music files
        files = [f async for f in scanner.find_music_files()]

        # Check that we didn't find any files in the inaccessible directory
        assert not any("inaccessible" in str(f) for f in files)
    finally:
        # Restore permissions
        inaccessible_dir.chmod(0o755)
