from pathlib import Path
from unittest.mock import AsyncMock, patch

import pytest

from mus.application.use_cases.full_scan_interactor import FullScanInteractor


@pytest.fixture
def track_repository():
    return AsyncMock()


@pytest.fixture
def scan_tracks_use_case():
    return AsyncMock()


@pytest.fixture
def db_path():
    return "/test/db/path.db"


@pytest.fixture
def covers_dir():
    return Path("/test/covers")


@pytest.fixture
def music_dir():
    return Path("/test/music")


@pytest.fixture
def interactor(track_repository, scan_tracks_use_case, db_path, covers_dir, music_dir):
    return FullScanInteractor(
        track_repository=track_repository,
        scan_tracks_use_case=scan_tracks_use_case,
        db_path=db_path,
        covers_dir=covers_dir,
        music_dir=music_dir,
    )


@pytest.mark.asyncio
async def test_execute_success(
    interactor, track_repository, scan_tracks_use_case, music_dir
):
    """Test successful execution of full scan process."""
    # Mock os.remove to simulate successful DB deletion
    with patch("os.remove") as mock_remove:
        await interactor.execute()

        # Verify DB file was removed
        mock_remove.assert_called_once_with("/test/db/path.db")

        # Verify schema was initialized
        track_repository.initialize_schema.assert_called_once()

        # Verify scan was triggered
        scan_tracks_use_case.execute.assert_called_once_with(music_dir)


@pytest.mark.asyncio
async def test_execute_db_not_found(
    interactor, track_repository, scan_tracks_use_case, music_dir
):
    """Test execution when DB file doesn't exist."""
    # Mock os.remove to raise FileNotFoundError
    with patch("os.remove", side_effect=FileNotFoundError):
        await interactor.execute()

        # Verify schema was still initialized
        track_repository.initialize_schema.assert_called_once()

        # Verify scan was still triggered
        scan_tracks_use_case.execute.assert_called_once_with(music_dir)


@pytest.mark.asyncio
async def test_clear_cover_cache_success(interactor, covers_dir):
    """Test successful clearing of cover cache."""
    mock_files = [
        covers_dir / "1_small.webp",
        covers_dir / "1_medium.webp",
        covers_dir / "2_small.webp",
    ]

    # Mock Path.exists and Path.glob
    with (
        patch.object(Path, "exists", return_value=True),
        patch.object(Path, "glob", return_value=mock_files),
        patch.object(Path, "unlink") as mock_unlink,
    ):
        await interactor._clear_cover_cache()

        # Verify each file was deleted
        assert mock_unlink.call_count == len(mock_files)


@pytest.mark.asyncio
async def test_clear_cover_cache_dir_not_exists(interactor):
    """Test when cover cache directory doesn't exist."""
    with (
        patch.object(Path, "exists", return_value=False),
        patch.object(Path, "glob") as mock_glob,
    ):
        await interactor._clear_cover_cache()

        # Verify glob was not called since directory doesn't exist
        mock_glob.assert_not_called()


@pytest.mark.asyncio
async def test_clear_cover_cache_unlink_error(interactor, covers_dir):
    """Test handling of file deletion errors."""
    mock_file = covers_dir / "error.webp"

    # Mock Path.exists, Path.glob, and Path.unlink
    with (
        patch.object(Path, "exists", return_value=True),
        patch.object(Path, "glob", return_value=[mock_file]),
        patch.object(Path, "unlink", side_effect=OSError("Test error")),
    ):
        # Should not raise exception
        await interactor._clear_cover_cache()


@pytest.mark.asyncio
async def test_execute_repository_error(interactor, track_repository):
    """Test handling of repository initialization error."""
    track_repository.initialize_schema.side_effect = Exception("DB error")

    with patch("os.remove"), pytest.raises(Exception) as exc_info:
        await interactor.execute()

    assert str(exc_info.value) == "DB error"
