from pathlib import Path
from unittest.mock import AsyncMock, MagicMock

import pytest

from mus.application.use_cases.scan_tracks import ScanTracksUseCase
from mus.domain.track import Track  # Import Track


class AsyncIteratorMock:
    def __init__(self, items):
        self.items = items
        self.index = 0

    def __aiter__(self):
        return self

    async def __anext__(self):
        if self.index >= len(self.items):
            raise StopAsyncIteration
        item = self.items[self.index]
        self.index += 1
        return item


@pytest.fixture
def file_scanner():
    scanner = MagicMock()
    # No need to mock find_music_files here, we'll set return_value in tests
    return scanner


@pytest.fixture
def metadata_reader():
    reader = AsyncMock()
    reader.read_metadata.return_value = ("Test Title", "Test Artist", 180, 1234567890)
    return reader


@pytest.fixture
def track_repository():
    repo = AsyncMock()
    repo.exists_by_path.return_value = False
    repo.add = AsyncMock()  # Ensure add is also an AsyncMock
    return repo


@pytest.fixture
def use_case(file_scanner, metadata_reader, track_repository):
    return ScanTracksUseCase(
        file_scanner=file_scanner,
        metadata_reader=metadata_reader,
        track_repository=track_repository,
    )


async def test_scan_tracks_success(
    use_case, file_scanner, metadata_reader, track_repository
):
    files = [Path("/test/file1.mp3"), Path("/test/file2.mp3")]
    file_scanner.find_music_files.return_value = AsyncIteratorMock(files)

    await use_case.execute(Path("/test/path"))

    assert file_scanner.find_music_files.call_count == 1
    # Correct the expected call count based on the number of files yielded
    assert metadata_reader.read_metadata.call_count == len(files)
    assert track_repository.add.call_count == len(files)
    # Check that add was called with the correct Track objects
    call_args_list = track_repository.add.call_args_list
    assert len(call_args_list) == 2
    added_track1 = call_args_list[0].args[0]
    added_track2 = call_args_list[1].args[0]
    assert isinstance(added_track1, Track)
    assert added_track1.file_path == files[0]
    assert added_track1.title == "Test Title"
    assert isinstance(added_track2, Track)
    assert added_track2.file_path == files[1]
    assert added_track2.title == "Test Title"


async def test_scan_tracks_skip_existing(
    use_case, file_scanner, metadata_reader, track_repository
):
    files = [Path("/test/file1.mp3"), Path("/test/file2.mp3")]
    file_scanner.find_music_files.return_value = AsyncIteratorMock(files)
    # Make exists_by_path return True for all files
    track_repository.exists_by_path.return_value = True

    await use_case.execute(Path("/test/path"))

    assert file_scanner.find_music_files.call_count == 1
    # exists_by_path should be called for each file
    assert track_repository.exists_by_path.call_count == len(files)
    # No metadata should be read or tracks added
    assert metadata_reader.read_metadata.call_count == 0
    assert track_repository.add.call_count == 0


async def test_scan_tracks_skip_invalid_metadata(
    use_case, file_scanner, metadata_reader, track_repository
):
    files = [Path("/test/file1.mp3"), Path("/test/file2.mp3")]
    file_scanner.find_music_files.return_value = AsyncIteratorMock(files)
    # Make metadata reader return None for all files
    metadata_reader.read_metadata.return_value = None

    await use_case.execute(Path("/test/path"))

    assert file_scanner.find_music_files.call_count == 1
    # exists_by_path should be called first
    assert track_repository.exists_by_path.call_count == len(files)
    # metadata_reader should be called for each file (since exists is False)
    assert metadata_reader.read_metadata.call_count == len(files)
    # No tracks should be added
    assert track_repository.add.call_count == 0
