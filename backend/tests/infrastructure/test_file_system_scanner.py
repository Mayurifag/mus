import os
import pytest
from pathlib import Path
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner


@pytest.fixture
def test_music_dir():
    """Create a temporary test music directory with nested folders and test files."""
    base_dir = Path("./test_music")

    # Create directory structure
    dirs = [
        base_dir,
        base_dir / "album1",
        base_dir / "album2",
        base_dir / "album2" / "cd1",
        base_dir / "album2" / "cd2",
        base_dir / "empty_dir",
    ]

    for directory in dirs:
        os.makedirs(directory, exist_ok=True)

    # Create test files
    test_files = [
        base_dir / "track1.mp3",
        base_dir / "track2.FLAC",  # Test case insensitivity
        base_dir / "ignore.txt",  # Should be ignored
        base_dir / "album1" / "track3.mp3",
        base_dir / "album1" / "track4.flac",
        base_dir / "album2" / "cd1" / "track5.mp3",
        base_dir / "album2" / "cd2" / "track6.flac",
        base_dir / "album2" / "cd2" / "track7.mp3",
    ]

    for file_path in test_files:
        # Create empty files
        with open(file_path, "w") as f:
            f.write("Test file")

    yield base_dir

    # Cleanup
    for file_path in test_files:
        if file_path.exists():
            os.remove(file_path)

    # Remove directories in reverse order (deepest first)
    for directory in reversed(dirs):
        if directory.exists():
            try:
                os.rmdir(directory)
            except OSError:
                # Directory might not be empty if test fails, so just ignore
                pass


@pytest.fixture
def file_system_scanner(test_music_dir):
    """Create a FileSystemScanner instance using the test directory."""
    scanner = FileSystemScanner()
    scanner.root_dir = test_music_dir
    return scanner


@pytest.fixture
def custom_music_dir():
    """Create a temporary custom music directory."""
    base_dir = Path("./custom_music")
    os.makedirs(base_dir, exist_ok=True)

    yield base_dir

    if base_dir.exists():
        try:
            os.rmdir(base_dir)
        except OSError:
            pass


def test_music_dir_environment_variable(monkeypatch, custom_music_dir):
    """Test that FileSystemScanner uses MUSIC_DIR environment variable."""
    custom_path = str(custom_music_dir)
    monkeypatch.setenv("MUSIC_DIR", custom_path)

    scanner = FileSystemScanner()
    assert scanner.MUSIC_DIR == custom_path
    assert scanner.root_dir == Path(custom_path)


def test_music_dir_default_value(monkeypatch):
    """Test that FileSystemScanner uses default music directory when MUSIC_DIR is not set."""
    monkeypatch.delenv("MUSIC_DIR", raising=False)

    scanner = FileSystemScanner()
    assert scanner.MUSIC_DIR == "./music"
    assert scanner.root_dir == Path("./music")


@pytest.mark.asyncio
async def test_find_music_files(file_system_scanner, test_music_dir):
    """Test finding music files in a directory."""
    music_files = []
    async for file_path in file_system_scanner.find_music_files(test_music_dir):
        music_files.append(file_path)

    # Should find 6 music files (5 mp3 and 1 flac)
    assert len(music_files) == 7

    # All found files should have supported extensions
    for file_path in music_files:
        assert file_path.suffix.lower() in file_system_scanner.SUPPORTED_EXTENSIONS


@pytest.mark.asyncio
async def test_find_music_files_with_custom_extensions(
    file_system_scanner, test_music_dir
):
    """Test finding music files with custom file extensions."""
    # Only look for MP3 files
    mp3_files = []
    async for file_path in file_system_scanner.find_music_files(
        test_music_dir, extensions={".mp3"}
    ):
        mp3_files.append(file_path)

    # Should find 4 mp3 files
    assert len(mp3_files) == 4

    # All found files should be MP3
    for file_path in mp3_files:
        assert file_path.suffix.lower() == ".mp3"

    # Only look for FLAC files
    flac_files = []
    async for file_path in file_system_scanner.find_music_files(
        test_music_dir, extensions={".flac"}
    ):
        flac_files.append(file_path)

    # Should find 3 flac files
    assert len(flac_files) == 3

    # All found files should be FLAC
    for file_path in flac_files:
        assert file_path.suffix.lower() == ".flac"


@pytest.mark.asyncio
async def test_scan_directories(file_system_scanner, test_music_dir):
    """Test scanning multiple directories."""
    # Create test directories
    dirs = [
        test_music_dir / "album1",
        test_music_dir / "album2",
    ]

    # Scan multiple directories
    all_files = []
    async for file_path in file_system_scanner.scan_directories(dirs):
        all_files.append(file_path)

    # Should find 5 files (2 in album1, 3 in album2 including subdirectories)
    assert len(all_files) == 5


@pytest.mark.asyncio
async def test_nonexistent_directory(file_system_scanner):
    """Test handling of nonexistent directories."""
    nonexistent_dir = Path("./dir_that_does_not_exist")

    # Should not raise an exception
    files = []
    async for file_path in file_system_scanner.find_music_files(nonexistent_dir):
        files.append(file_path)

    # Should find 0 files
    assert len(files) == 0


@pytest.mark.asyncio
async def test_empty_directory(file_system_scanner, test_music_dir):
    """Test handling of empty directories."""
    empty_dir = test_music_dir / "empty_dir"

    # Should not raise an exception
    files = []
    async for file_path in file_system_scanner.find_music_files(empty_dir):
        files.append(file_path)

    # Should find 0 files
    assert len(files) == 0


@pytest.mark.asyncio
async def test_scan_directories_default(file_system_scanner):
    """Test that scan_directories defaults to the music directory when no dirs provided."""
    all_files = []
    async for file_path in file_system_scanner.scan_directories():
        all_files.append(file_path)

    # Should find all 7 music files
    assert len(all_files) == 7
