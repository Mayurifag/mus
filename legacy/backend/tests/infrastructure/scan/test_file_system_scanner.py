from pathlib import Path

import pytest

from mus.infrastructure.scan.file_system_scanner import FileSystemScanner


@pytest.fixture
def music_dir(tmp_path: Path) -> Path:
    """Create a temporary directory structure for music files using tmp_path."""
    music_path = tmp_path / "music"
    music_path.mkdir()

    # Create test files
    (music_path / "test1.mp3").touch()
    (music_path / "test2.flac").touch()
    (music_path / "test3.txt").touch()  # Non-music file

    # Create subdirectory with more files
    sub_dir = music_path / "subdir"
    sub_dir.mkdir()
    (sub_dir / "test4.mp3").touch()
    (sub_dir / "test5.ogg").touch()

    return music_path


async def test_find_music_files(music_dir: Path):
    scanner = FileSystemScanner(music_dir.parent)  # Scan from the parent of 'music'
    files = [f async for f in scanner.find_music_files(music_dir)]

    # Sort results for consistent comparison
    file_paths = sorted([f.relative_to(music_dir.parent) for f in files])

    assert file_paths == sorted(
        [
            Path("music/test1.mp3"),
            Path("music/test2.flac"),
            Path("music/subdir/test4.mp3"),
            Path("music/subdir/test5.ogg"),
        ]
    )
    assert all(f.suffix in FileSystemScanner.SUPPORTED_EXTENSIONS for f in files)
