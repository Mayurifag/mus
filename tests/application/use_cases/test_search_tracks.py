from pathlib import Path
from unittest.mock import AsyncMock

import pytest

from mus.application.use_cases.search_tracks import SearchTracksUseCase
from mus.domain.track import Track


@pytest.fixture
def track_repository():
    repo = AsyncMock()
    repo.search_by_title.return_value = [
        Track(
            title="Test Track",
            artist="Test Artist",
            duration=180,
            file_path=Path("/test/path/track1.mp3"),
            added_at=1704067200,  # 2024-01-01 00:00:00 UTC
        )
    ]
    repo.get_all.return_value = [
        Track(
            title="Test Track",
            artist="Test Artist",
            duration=180,
            file_path=Path("/test/path/track1.mp3"),
            added_at=1704067200,  # 2024-01-01 00:00:00 UTC
        ),
        Track(
            title="Another Track",
            artist="Another Artist",
            duration=240,
            file_path=Path("/test/path/track2.mp3"),
            added_at=1704067200,  # 2024-01-01 00:00:00 UTC
        ),
    ]
    return repo


@pytest.mark.asyncio
async def test_search_by_title(track_repository):
    use_case = SearchTracksUseCase(track_repository)
    results = await use_case.search_by_title("Test")

    assert track_repository.search_by_title.called
    assert len(results) == 1
    assert results[0].title == "Test Track"


@pytest.mark.asyncio
async def test_get_all(track_repository):
    use_case = SearchTracksUseCase(track_repository)
    results = await use_case.get_all()

    assert track_repository.get_all.called
    assert len(results) == 2
    assert results[0].title == "Test Track"
    assert results[1].title == "Another Track"
