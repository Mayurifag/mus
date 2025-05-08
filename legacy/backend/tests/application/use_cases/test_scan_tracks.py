from pathlib import Path
from unittest.mock import AsyncMock, MagicMock

import pytest

from mus.application.use_cases.scan_tracks import ScanTracksUseCase


class AsyncIteratorMock:
    def __init__(self, items):
        self.items = items.copy()  # Create a copy to avoid modifying the original list

    def __aiter__(self):
        return self

    async def __anext__(self):
        try:
            return self.items.pop(0)
        except IndexError as err:
            raise StopAsyncIteration from err


@pytest.fixture
def mock_file_scanner():
    scanner = MagicMock()
    scanner.find_music_files = MagicMock()  # Changed from AsyncMock to MagicMock
    return scanner


@pytest.fixture
def mock_metadata_reader():
    reader = MagicMock()
    reader.read_metadata = AsyncMock()
    return reader


@pytest.fixture
def mock_track_repository():
    repo = MagicMock()
    repo.exists_by_path = AsyncMock()
    repo.add = AsyncMock()
    repo.set_cover_flag = AsyncMock()
    return repo


@pytest.fixture
def mock_cover_processor():
    processor = MagicMock()
    processor.process_and_save_cover = AsyncMock()
    return processor


@pytest.fixture
def use_case(
    mock_file_scanner, mock_metadata_reader, mock_track_repository, mock_cover_processor
):
    return ScanTracksUseCase(
        file_scanner=mock_file_scanner,
        metadata_reader=mock_metadata_reader,
        track_repository=mock_track_repository,
        cover_processor=mock_cover_processor,
    )


async def test_scan_tracks_success(
    use_case,
    mock_file_scanner,
    mock_metadata_reader,
    mock_track_repository,
    mock_cover_processor,
):
    # Setup
    test_file = Path("/test/file.mp3")
    mock_file_scanner.find_music_files.return_value = AsyncIteratorMock([test_file])
    mock_track_repository.exists_by_path.return_value = False
    mock_track_repository.add.return_value = 1
    mock_metadata_reader.read_metadata.return_value = (
        "Title",
        "Artist",
        180,
        1234567890,
        b"cover_data",
    )
    mock_cover_processor.process_and_save_cover.return_value = True

    # Execute
    await use_case.execute(Path("/test"))

    # Verify
    mock_file_scanner.find_music_files.assert_called_once()
    mock_track_repository.exists_by_path.assert_called_once_with(test_file)
    mock_metadata_reader.read_metadata.assert_called_once_with(test_file)
    mock_track_repository.add.assert_called_once()
    mock_cover_processor.process_and_save_cover.assert_called_once_with(
        1, b"cover_data"
    )
    mock_track_repository.set_cover_flag.assert_called_once_with(1, True)


async def test_scan_tracks_skip_existing(
    use_case, mock_file_scanner, mock_metadata_reader, mock_track_repository
):
    # Setup
    test_file = Path("/test/file.mp3")
    mock_file_scanner.find_music_files.return_value = AsyncIteratorMock([test_file])
    mock_track_repository.exists_by_path.return_value = True

    # Execute
    await use_case.execute(Path("/test"))

    # Verify
    mock_file_scanner.find_music_files.assert_called_once()
    mock_track_repository.exists_by_path.assert_called_once_with(test_file)
    mock_metadata_reader.read_metadata.assert_not_called()
    mock_track_repository.add.assert_not_called()


async def test_scan_tracks_skip_invalid_metadata(
    use_case, mock_file_scanner, mock_metadata_reader, mock_track_repository
):
    # Setup
    test_file = Path("/test/file.mp3")
    mock_file_scanner.find_music_files.return_value = AsyncIteratorMock([test_file])
    mock_track_repository.exists_by_path.return_value = False
    mock_metadata_reader.read_metadata.return_value = None

    # Execute
    await use_case.execute(Path("/test"))

    # Verify
    mock_file_scanner.find_music_files.assert_called_once()
    mock_track_repository.exists_by_path.assert_called_once_with(test_file)
    mock_metadata_reader.read_metadata.assert_called_once_with(test_file)
    mock_track_repository.add.assert_not_called()
