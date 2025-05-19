import os
import pytest
from pathlib import Path
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
import asyncio
import time
from unittest.mock import patch
import logging


@pytest.fixture
def test_music_dir():
    """Create a temporary test music directory with nested folders and test files."""
    base_dir = Path("./test_music")
    dirs_to_create = [
        base_dir,
        base_dir / "album1",
        base_dir / "album2",
        base_dir / "album2" / "cd1",
        base_dir / "album2" / "cd2",
        base_dir / "empty_dir",
    ]
    for d in dirs_to_create:
        os.makedirs(d, exist_ok=True)

    files_to_create = {
        base_dir / "track1.mp3": "",
        base_dir / "track2.FLAC": "",
        base_dir / "ignore.txt": "text",
        base_dir / "album1" / "track3.mp3": "",
        base_dir / "album1" / "track4.flac": "",
        base_dir / "album2" / "cd1" / "track5.mp3": "",
        base_dir / "album2" / "cd2" / "track6.flac": "",
        base_dir / "album2" / "cd2" / "track7.mp3": "",
    }
    for f_path, content in files_to_create.items():
        f_path.write_text(content)
    yield base_dir
    for f_path in files_to_create:
        if f_path.exists():
            os.remove(f_path)
    for d in reversed(dirs_to_create):
        if d.exists():
            try:
                os.rmdir(d)
            except OSError:
                pass


@pytest.fixture
def file_system_scanner():
    """Return a FileSystemScanner instance."""
    return FileSystemScanner()


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
    custom_path_str = str(custom_music_dir)
    monkeypatch.setenv("MUSIC_DIR", custom_path_str)

    scanner = FileSystemScanner()
    # Test the resolved Path object
    assert scanner.default_root_dir == custom_music_dir
    # Optionally, test the string attribute if it's still relevant for other reasons
    assert scanner.default_music_dir_str == custom_path_str


def test_music_dir_default_value(monkeypatch):
    """Test that FileSystemScanner uses default music directory when MUSIC_DIR is not set."""
    monkeypatch.delenv("MUSIC_DIR", raising=False)

    scanner = FileSystemScanner()
    # Test the resolved Path object against the default
    assert scanner.default_root_dir == Path("./music")
    # Optionally, test the string attribute
    assert scanner.default_music_dir_str == "./music"


@pytest.mark.asyncio
async def test_find_music_files(file_system_scanner, test_music_dir):
    """Test finding music files in a directory."""
    music_files = []
    async for file_path in file_system_scanner.scan_directories([test_music_dir]):
        music_files.append(file_path)

    expected_music_files = {
        test_music_dir / "track1.mp3",
        test_music_dir / "track2.FLAC",
        test_music_dir / "album1" / "track3.mp3",
        test_music_dir / "album1" / "track4.flac",
        test_music_dir / "album2" / "cd1" / "track5.mp3",
        test_music_dir / "album2" / "cd2" / "track6.flac",
        test_music_dir / "album2" / "cd2" / "track7.mp3",
    }
    assert set(music_files) == expected_music_files
    assert len(music_files) == 7


@pytest.mark.asyncio
async def test_find_music_files_with_custom_extensions(
    file_system_scanner, test_music_dir
):
    """Test finding music files with custom file extensions."""
    # Only look for MP3 files
    mp3_files = []
    # Changed find_music_files to scan_directories
    async for file_path in file_system_scanner.scan_directories(
        [test_music_dir], extensions={".mp3"}
    ):
        mp3_files.append(file_path)

    # Assertions based on the content of test_music_dir fixture
    # (Assuming it contains 4 MP3 files: sample_1, sample_3, sample_5, sample_6)
    assert len(mp3_files) == 4
    assert all(f.suffix == ".mp3" for f in mp3_files)


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
    # Changed find_music_files to scan_directories
    async for file_path in file_system_scanner.scan_directories([nonexistent_dir]):
        files.append(file_path)
    assert not files  # Expect no files from a non-existent directory


@pytest.mark.asyncio
async def test_empty_directory(file_system_scanner, test_music_dir):
    """Test handling of empty directories."""
    empty_dir = test_music_dir / "empty_dir"
    # Ensure the empty directory exists for the test
    empty_dir.mkdir(exist_ok=True)

    # Should not raise an exception
    files = []
    # Changed find_music_files to scan_directories
    async for file_path in file_system_scanner.scan_directories([empty_dir]):
        files.append(file_path)
    assert not files  # Expect no files from an empty directory


@pytest.mark.asyncio
async def test_scan_directories_default(
    file_system_scanner, test_music_dir, monkeypatch
):
    """Test that scan_directories defaults to the music directory when no dirs provided."""
    monkeypatch.setenv("MUSIC_DIR", str(test_music_dir))
    scanner_with_patched_env = FileSystemScanner()

    all_files = []
    async for file_path in scanner_with_patched_env.scan_directories():
        all_files.append(file_path)

    expected_music_files = {
        test_music_dir / "track1.mp3",
        test_music_dir / "track2.FLAC",
        test_music_dir / "album1" / "track3.mp3",
        test_music_dir / "album1" / "track4.flac",
        test_music_dir / "album2" / "cd1" / "track5.mp3",
        test_music_dir / "album2" / "cd2" / "track6.flac",
        test_music_dir / "album2" / "cd2" / "track7.mp3",
    }
    assert set(all_files) == expected_music_files
    assert len(all_files) == 7


@pytest.fixture
def temp_music_dir_for_simple_scanner(tmp_path: Path) -> Path:
    music_dir = tmp_path / "music_lean"
    music_dir.mkdir(parents=True, exist_ok=True)

    (music_dir / "song1.mp3").write_text("s1")
    time.sleep(0.01)
    (music_dir / "song2.flac").write_text("s2")
    (music_dir / "other.txt").write_text("txt")

    sub_dir = music_dir / "subdir"
    sub_dir.mkdir(parents=True, exist_ok=True)
    (sub_dir / "song3.mp3").write_text("s3")
    time.sleep(0.01)
    (sub_dir / "song4.flac").write_text("s4")

    empty_subdir = music_dir / "empty_subdir"
    empty_subdir.mkdir(parents=True, exist_ok=True)

    (music_dir / "old_song.mp3").write_text("s_old")
    two_days_ago = time.time() - 2 * 86400
    os.utime(music_dir / "old_song.mp3", (two_days_ago, two_days_ago))

    return music_dir


@pytest.fixture
def scanner_lean(
    temp_music_dir_for_simple_scanner: Path, monkeypatch: pytest.MonkeyPatch
) -> FileSystemScanner:
    monkeypatch.setenv("MUSIC_DIR", str(temp_music_dir_for_simple_scanner))
    fss = FileSystemScanner()
    # fss.root_dir = temp_music_dir_for_simple_scanner # Removed this incorrect line
    return fss


async def collect_results_lean(agen):
    return [item async for item in agen]


@pytest.mark.asyncio
async def test_scan_basic_lean(
    scanner_lean: FileSystemScanner, temp_music_dir_for_simple_scanner: Path
):
    files_found = await collect_results_lean(scanner_lean.scan_directories())
    expected_files = {
        temp_music_dir_for_simple_scanner / "song1.mp3",
        temp_music_dir_for_simple_scanner / "song2.flac",
        temp_music_dir_for_simple_scanner / "subdir" / "song3.mp3",
        temp_music_dir_for_simple_scanner / "subdir" / "song4.flac",
        temp_music_dir_for_simple_scanner / "old_song.mp3",
    }
    assert set(files_found) == expected_files


@pytest.mark.asyncio
async def test_scan_with_specific_extensions_lean(
    scanner_lean: FileSystemScanner, temp_music_dir_for_simple_scanner: Path
):
    files_found = await collect_results_lean(
        scanner_lean.scan_directories(extensions={".mp3"})
    )
    expected_files = {
        temp_music_dir_for_simple_scanner / "song1.mp3",
        temp_music_dir_for_simple_scanner / "subdir" / "song3.mp3",
        temp_music_dir_for_simple_scanner / "old_song.mp3",
    }
    assert set(files_found) == expected_files


@pytest.mark.asyncio
async def test_scan_with_min_mtime_lean(
    scanner_lean: FileSystemScanner, temp_music_dir_for_simple_scanner: Path
):
    one_day_ago_ts = int(time.time() - 1 * 86400)
    files_found = await collect_results_lean(
        scanner_lean.scan_directories(min_mtime=one_day_ago_ts)
    )
    expected_files = {
        temp_music_dir_for_simple_scanner / "song1.mp3",
        temp_music_dir_for_simple_scanner / "song2.flac",
        temp_music_dir_for_simple_scanner / "subdir" / "song3.mp3",
        temp_music_dir_for_simple_scanner / "subdir" / "song4.flac",
    }
    assert set(files_found) == expected_files
    assert (temp_music_dir_for_simple_scanner / "old_song.mp3") not in files_found


@pytest.mark.asyncio
async def test_scan_empty_directory_lean(
    scanner_lean: FileSystemScanner, temp_music_dir_for_simple_scanner: Path
):
    empty_dir_path = temp_music_dir_for_simple_scanner / "empty_subdir"
    files_found = await collect_results_lean(
        scanner_lean.scan_directories(directories=[empty_dir_path])
    )
    assert len(files_found) == 0


@pytest.mark.asyncio
async def test_scan_non_existent_directory_lean(
    scanner_lean: FileSystemScanner, temp_music_dir_for_simple_scanner: Path
):
    non_existent_path = temp_music_dir_for_simple_scanner / "does_not_exist"
    # The simplified scanner returns empty if dir does not exist/is not dir, without logging.
    files_found = await collect_results_lean(
        scanner_lean.scan_directories(directories=[non_existent_path])
    )
    assert len(files_found) == 0


@pytest.mark.asyncio
async def test_scan_permission_error_raises_lean(
    scanner_lean: FileSystemScanner, temp_music_dir_for_simple_scanner: Path, caplog
):
    target_dir_to_fail = temp_music_dir_for_simple_scanner / "subdir_perm_error"
    target_dir_to_fail.mkdir()
    (target_dir_to_fail / "internal.mp3").write_text("content")

    original_asyncio_to_thread = asyncio.to_thread

    async def mock_asyncio_to_thread_for_exception(
        func_passed, *args_for_func, **kwargs_for_func
    ):
        if (
            hasattr(func_passed, "__name__")
            and func_passed.__name__ == "iterdir"
            and hasattr(func_passed, "__self__")
            and isinstance(func_passed.__self__, Path)
            and str(func_passed.__self__) == str(target_dir_to_fail)
        ):
            raise PermissionError(
                f"Simulated PermissionError on iterdir for {func_passed.__self__}"
            )
        return await original_asyncio_to_thread(
            func_passed, *args_for_func, **kwargs_for_func
        )

    caplog.set_level(
        logging.WARNING, logger="src.mus.infrastructure.scanner.file_system_scanner"
    )

    with patch(
        "src.mus.infrastructure.scanner.file_system_scanner.asyncio.to_thread",  # Corrected patch path
        new=mock_asyncio_to_thread_for_exception,
    ):
        # The scanner should log the warning and continue, not raise the PermissionError
        files_found = await collect_results_lean(
            scanner_lean.scan_directories(
                directories=[temp_music_dir_for_simple_scanner]
            )
        )
        # Check that the warning was logged
        assert any(
            f"Cannot read directory contents {target_dir_to_fail}" in record.message
            and "Simulated PermissionError" in record.message
            and record.levelname == "WARNING"
            for record in caplog.records
        )
        # Check that the file inside the permission-denied directory was not scanned
        assert not any(target_dir_to_fail / "internal.mp3" == p for p in files_found)
        # Check that other files were scanned (example: song1.mp3 from the root of temp_music_dir)
        assert any(
            temp_music_dir_for_simple_scanner / "song1.mp3" == p for p in files_found
        )


@pytest.mark.asyncio
async def test_scan_multiple_directories_lean(
    scanner_lean: FileSystemScanner, tmp_path: Path
):
    dir1 = tmp_path / "music_set1_lean"
    dir1.mkdir()
    (dir1 / "track1.mp3").write_text("d1t1")

    dir2 = tmp_path / "music_set2_lean"
    dir2.mkdir()
    (dir2 / "track2.flac").write_text("d2t2")

    sub_dir2 = dir2 / "sub"
    sub_dir2.mkdir(parents=True, exist_ok=True)
    (sub_dir2 / "track3.mp3").write_text("d2st3")

    files_found = await collect_results_lean(
        scanner_lean.scan_directories(directories=[dir1, dir2])
    )
    expected = {
        dir1 / "track1.mp3",
        dir2 / "track2.flac",
        sub_dir2 / "track3.mp3",
    }
    assert set(files_found) == expected


@pytest.mark.asyncio
async def test_file_is_actually_a_directory_lean(
    scanner_lean: FileSystemScanner, temp_music_dir_for_simple_scanner: Path
):
    file_as_dir = temp_music_dir_for_simple_scanner / "looks_like_file.mp3"
    file_as_dir.mkdir()
    files_found = await collect_results_lean(
        scanner_lean.scan_directories(directories=[temp_music_dir_for_simple_scanner])
    )
    assert file_as_dir not in files_found
    expected_files = {
        temp_music_dir_for_simple_scanner / "song1.mp3",
        temp_music_dir_for_simple_scanner / "song2.flac",
        temp_music_dir_for_simple_scanner / "subdir" / "song3.mp3",
        temp_music_dir_for_simple_scanner / "subdir" / "song4.flac",
        temp_music_dir_for_simple_scanner / "old_song.mp3",
    }
    assert expected_files.issubset(set(files_found))
    assert len(files_found) == len(expected_files)
