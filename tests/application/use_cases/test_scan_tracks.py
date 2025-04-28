from datetime import datetime
from pathlib import Path
from unittest.mock import AsyncMock

import pytest

from mus.application.use_cases.scan_tracks import ScanTracksUseCase


class AsyncIteratorMock:
    def __init__(self, items):
        self.items = items.copy()

    def __aiter__(self):
        return self

    async def __anext__(self):
        try:
            return self.items.pop(0)
        except IndexError as err:
            raise StopAsyncIteration from err


@pytest.fixture
def file_scanner():
    scanner = AsyncMock()
    items = [
        Path("/test/path/track1.mp3"),
        Path("/test/path/track2.mp3"),
    ]
    scanner.find_music_files = AsyncMock(side_effect=lambda _: AsyncIteratorMock(items))
    return scanner


@pytest.fixture
def metadata_reader():
    reader = AsyncMock()
    reader.read_metadata.return_value = (
        "Test Title",
        "Test Artist",
        180.0,
        datetime(2024, 1, 1),
    )
    return reader


@pytest.fixture
def track_repository():
    repo = AsyncMock()
    repo.exists_by_path.return_value = False
    return repo


@pytest.mark.asyncio
async def test_scan_tracks_success(file_scanner, metadata_reader, track_repository):
    use_case = ScanTracksUseCase(file_scanner, metadata_reader, track_repository)
    await use_case.execute(Path("/test/path"))

    assert file_scanner.find_music_files.called
    assert metadata_reader.read_metadata.call_count == 2
    assert track_repository.add.call_count == 2
    assert track_repository.exists_by_path.call_count == 2


@pytest.mark.asyncio
async def test_scan_tracks_skip_existing(
    file_scanner, metadata_reader, track_repository
):
    track_repository.exists_by_path.return_value = True
    use_case = ScanTracksUseCase(file_scanner, metadata_reader, track_repository)
    await use_case.execute(Path("/test/path"))

    assert file_scanner.find_music_files.called
    assert metadata_reader.read_metadata.call_count == 0
    assert track_repository.add.call_count == 0
    assert track_repository.exists_by_path.call_count == 2


@pytest.mark.asyncio
async def test_scan_tracks_skip_invalid_metadata(
    file_scanner, metadata_reader, track_repository
):
    metadata_reader.read_metadata.return_value = None
    use_case = ScanTracksUseCase(file_scanner, metadata_reader, track_repository)
    await use_case.execute(Path("/test/path"))

    assert file_scanner.find_music_files.called
    assert metadata_reader.read_metadata.call_count == 2
    assert track_repository.add.call_count == 0
    assert track_repository.exists_by_path.call_count == 2
