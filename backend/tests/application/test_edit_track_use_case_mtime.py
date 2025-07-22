import os
import time
import pytest
import pytest_asyncio
from pathlib import Path
from unittest.mock import patch, AsyncMock, Mock
from src.mus.application.use_cases.edit_track_use_case import EditTrackUseCase
from src.mus.application.dtos.track import TrackUpdateDTO
from src.mus.domain.entities.track import Track
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


@pytest_asyncio.fixture
async def mock_track_repo():
    repo = Mock(spec=SQLiteTrackRepository)

    # Create async methods that return values directly
    async def mock_get_by_id(_track_id):
        return repo._get_by_id_return_value

    async def mock_update(track):
        return track

    async def mock_commit():
        pass

    async def mock_refresh(_track):
        pass

    repo.get_by_id = mock_get_by_id
    repo.update = mock_update
    repo.session = Mock()
    repo.session.commit = mock_commit
    repo.session.refresh = mock_refresh

    return repo


@pytest_asyncio.fixture
async def sample_track(tmp_path: Path):
    # Create a temporary audio file
    audio_file = tmp_path / "test.mp3"
    audio_file.write_text("fake audio content")

    # Set a specific mtime
    original_time = time.time() - 3600  # 1 hour ago
    os.utime(audio_file, (original_time, original_time))

    track = Track(
        id=1,
        title="Original Title",
        artist="Original Artist",
        duration=180,
        file_path=str(audio_file),
        added_at=int(time.time()),
        updated_at=int(time.time()),
    )
    return track, original_time


@pytest.mark.asyncio
async def test_edit_track_preserves_mtime(mock_track_repo, sample_track, fake_redis):
    track, _ = sample_track
    mock_track_repo._get_by_id_return_value = track

    use_case = EditTrackUseCase(mock_track_repo)
    update_data = TrackUpdateDTO(title="New Title")

    # Mock mutagen to avoid actual file operations
    with (
        patch(
            "src.mus.application.use_cases.edit_track_use_case.MutagenFile"
        ) as mock_mutagen,
        patch(
            "src.mus.application.use_cases.edit_track_use_case.broadcast_sse_event"
        ) as mock_broadcast,
        patch(
            "src.mus.application.use_cases.edit_track_use_case.create_track_dto_with_covers"
        ) as mock_create_dto,
    ):
        # Setup mocks
        mock_audio = AsyncMock()
        mock_audio.save = Mock(return_value=None)  # Use Mock instead of AsyncMock
        mock_mutagen.return_value = mock_audio
        mock_broadcast.return_value = None
        mock_dto = Mock()
        mock_dto.model_dump.return_value = {"id": 1, "title": "Test Track"}
        mock_create_dto.return_value = mock_dto

        # Get original mtime before the operation
        original_stat = os.stat(track.file_path)
        original_file_mtime = original_stat.st_mtime

        # Execute the use case
        result = await use_case.execute(track.id, update_data)

        # Verify the result
        assert result["status"] == "success"

        # Verify that the file mtime was preserved
        final_stat = os.stat(track.file_path)
        final_file_mtime = final_stat.st_mtime

        # The mtime should be the same as before (within a small tolerance for floating point precision)
        assert abs(final_file_mtime - original_file_mtime) < 0.1

        # Verify that mutagen save was called
        mock_audio.save.assert_called_once()


@pytest.mark.asyncio
async def test_edit_track_no_changes_no_mtime_operation(
    mock_track_repo, sample_track, fake_redis
):
    track, _ = sample_track
    mock_track_repo._get_by_id_return_value = track

    use_case = EditTrackUseCase(mock_track_repo)
    # Update with the same title (no actual changes)
    update_data = TrackUpdateDTO(title=track.title)

    with patch(
        "src.mus.application.use_cases.edit_track_use_case.MutagenFile"
    ) as mock_mutagen:
        mock_audio = AsyncMock()
        mock_audio.save = Mock(return_value=None)  # Use Mock instead of AsyncMock
        mock_mutagen.return_value = mock_audio

        # Execute the use case
        result = await use_case.execute(track.id, update_data)

        # Verify no changes were made
        assert result["status"] == "no_changes"

        # Verify that mutagen save was NOT called since there were no changes
        mock_audio.save.assert_not_called()
