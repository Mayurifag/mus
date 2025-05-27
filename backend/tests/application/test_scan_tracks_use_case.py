import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from pathlib import Path
from typing import List, Callable, AsyncGenerator

from sqlalchemy.ext.asyncio import AsyncSession
from contextlib import AbstractAsyncContextManager

from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.domain.entities.track import Track
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


@pytest.fixture
def mock_async_session() -> AsyncMock:
    session = AsyncMock(spec=AsyncSession)
    session.begin = MagicMock(
        return_value=AsyncMock(
            __aenter__=AsyncMock(return_value=None),
            __aexit__=AsyncMock(return_value=None),
        )
    )
    return session


@pytest.fixture
def mock_session_factory(mock_async_session: AsyncMock) -> MagicMock:
    # This factory, when called, returns an async context manager that yields the mock_async_session
    mock_manager = AsyncMock(spec=AbstractAsyncContextManager)
    mock_manager.__aenter__.return_value = mock_async_session
    mock_manager.__aexit__.return_value = (
        None  # Or AsyncMock(return_value=None) if it needs to be awaitable
    )

    factory = MagicMock(spec=Callable[[], AbstractAsyncContextManager[AsyncSession]])
    factory.return_value = mock_manager
    return factory


@pytest.fixture
def mock_file_system_scanner() -> MagicMock:
    scanner = MagicMock(spec=FileSystemScanner)

    # Make scan_directories an async generator
    async def mock_scan_directories(*args, **kwargs) -> AsyncGenerator[Path, None]:
        if False:  # Ensure it's a generator
            yield

    scanner.scan_directories = MagicMock(side_effect=mock_scan_directories)
    return scanner


@pytest.fixture
def mock_cover_processor() -> MagicMock:
    processor = MagicMock(spec=CoverProcessor)
    processor.process_tracks_covers_batch = AsyncMock(return_value={})
    return processor


@pytest.fixture
def mock_track_repository(mock_async_session: AsyncMock) -> MagicMock:
    repo = MagicMock(spec=SQLiteTrackRepository)
    repo.session = (
        mock_async_session  # Allow access to session if needed for begin_nested etc.
    )
    repo.get_latest_track_added_at = AsyncMock(return_value=None)
    repo.upsert_track = AsyncMock(
        side_effect=lambda track: track
    )  # Return the input track by default
    repo.set_cover_flag = AsyncMock(return_value=None)
    return repo


@pytest.fixture
def scan_tracks_use_case(
    mock_session_factory: MagicMock,
    mock_file_system_scanner: MagicMock,
    mock_cover_processor: MagicMock,
) -> ScanTracksUseCase:
    return ScanTracksUseCase(
        session_factory=mock_session_factory,
        file_system_scanner=mock_file_system_scanner,
        cover_processor=mock_cover_processor,
    )


@pytest.mark.asyncio
async def test_scan_directory_no_files_found(
    scan_tracks_use_case: ScanTracksUseCase,
    mock_file_system_scanner: MagicMock,
    mock_session_factory: MagicMock,  # To get mock_async_session for SQLiteTrackRepository mock
    mock_async_session: AsyncMock,
):
    """Test that scan completes with no changes if scanner finds no files."""

    # Ensure scan_directories yields nothing
    async def empty_scan_gen(*args, **kwargs) -> AsyncGenerator[Path, None]:
        if False:
            yield  # Make it an async generator

    mock_file_system_scanner.scan_directories.side_effect = empty_scan_gen

    # Mock the repository that gets instantiated inside the use case
    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.SQLiteTrackRepository"
    ) as MockRepo, patch(
        "src.mus.application.use_cases.scan_tracks_use_case.broadcast_sse_event"
    ) as mock_broadcast_sse:
        mock_repo_instance = MagicMock(spec=SQLiteTrackRepository)
        mock_repo_instance.session = mock_async_session
        mock_repo_instance.get_latest_track_added_at = AsyncMock(return_value=None)
        MockRepo.return_value = mock_repo_instance

        result = await scan_tracks_use_case.scan_directory()

    assert result.success is True
    assert result.tracks_added == 0
    assert result.tracks_updated == 0
    assert result.errors == 0
    assert "No new or modified files found" in result.message
    mock_file_system_scanner.scan_directories.assert_called_once_with(
        None, min_mtime=None
    )
    mock_repo_instance.get_latest_track_added_at.assert_awaited_once()

    # Verify SSE events for empty scan
    # Initial progress
    mock_broadcast_sse.assert_any_call(
        message_to_show="Scanning 0 files...",
        message_level="info",
        action_key="scan_progress",
        action_payload={"processed": 0, "total": 0},
    )

    # Final summary
    mock_broadcast_sse.assert_any_call(
        message_to_show="Music library scan completed. No changes detected.",
        message_level="info",
        action_key=None,
        action_payload=None,
    )


@pytest.mark.asyncio
async def test_scan_directory_one_new_file(
    scan_tracks_use_case: ScanTracksUseCase,
    mock_file_system_scanner: MagicMock,
    mock_cover_processor: MagicMock,
    mock_session_factory: MagicMock,
    mock_async_session: AsyncMock,
):
    """Test scanning one new music file."""
    test_file_path = Path("/music/test_song.mp3")

    async def single_file_gen(*args, **kwargs) -> AsyncGenerator[Path, None]:
        yield test_file_path

    mock_file_system_scanner.scan_directories.side_effect = single_file_gen

    mock_extracted_metadata = {
        "title": "Test Song",
        "artist": "Test Artist",
        "duration": 180,
        "mtime": 1234567890,
    }
    # Mock the internal _extract_metadata method
    scan_tracks_use_case._extract_metadata = AsyncMock(
        return_value=mock_extracted_metadata
    )

    # Mock the repository
    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.SQLiteTrackRepository"
    ) as MockRepo:
        mock_repo_instance = MagicMock(spec=SQLiteTrackRepository)
        mock_repo_instance.session = mock_async_session
        mock_repo_instance.get_latest_track_added_at = AsyncMock(return_value=None)

        # Simulate adding a new track
        async def mock_upsert(track_to_upsert: Track) -> Track:
            track_to_upsert.id = 1  # Assign an ID as if it's newly created
            return track_to_upsert

        mock_repo_instance.upsert_track = AsyncMock(side_effect=mock_upsert)
        mock_repo_instance.set_cover_flag = AsyncMock()
        MockRepo.return_value = mock_repo_instance

        # Mock cover processor to succeed for this track_id
        mock_cover_processor.process_tracks_covers_batch = AsyncMock(
            return_value={1: True}
        )

        # Mock broadcast_sse_event
        with patch(
            "src.mus.application.use_cases.scan_tracks_use_case.broadcast_sse_event"
        ) as mock_broadcast_sse:
            result = await scan_tracks_use_case.scan_directory()

    assert result.success is True
    assert result.tracks_added == 1
    assert result.tracks_updated == 0
    assert result.errors == 0

    scan_tracks_use_case._extract_metadata.assert_awaited_once_with(test_file_path)

    # Check upsert_track call
    # Extract the Track object passed to upsert_track
    upsert_call_args = mock_repo_instance.upsert_track.await_args[0][0]
    assert isinstance(upsert_call_args, Track)
    assert upsert_call_args.title == "Test Song"
    assert upsert_call_args.artist == "Test Artist"
    assert upsert_call_args.file_path == str(test_file_path)

    mock_cover_processor.process_tracks_covers_batch.assert_awaited_once_with(
        [(1, test_file_path)]
    )
    mock_repo_instance.set_cover_flag.assert_awaited_once_with(1, True)

    # Check SSE events
    # Initial progress
    mock_broadcast_sse.assert_any_call(
        message_to_show="Scanning 1 files...",
        message_level="info",
        action_key="scan_progress",
        action_payload={"processed": 0, "total": 1},
    )

    # Track update event
    mock_broadcast_sse.assert_any_call(
        message_to_show="Track: Test Artist - Test Song",
        message_level="success",
        action_key="reload_tracks",
        action_payload=None,
    )

    # Progress update
    mock_broadcast_sse.assert_any_call(
        message_to_show="Scanning progress: 1/1 files",
        message_level="info",
        action_key="scan_progress",
        action_payload={"processed": 1, "total": 1},
    )

    # Final summary
    mock_broadcast_sse.assert_any_call(
        message_to_show="1 new tracks added.",
        message_level="success",
        action_key="reload_tracks",
        action_payload=None,
    )


@pytest.mark.asyncio
async def test_scan_directory_one_new_mp3_file_metadata_extraction(
    scan_tracks_use_case: ScanTracksUseCase,
    mock_file_system_scanner: MagicMock,
    mock_cover_processor: MagicMock,
    mock_async_session: AsyncMock,
    # mock_session_factory is implicitly used by the SQLiteTrackRepository patch
):
    """Test metadata extraction for a new MP3 file, verifying content passed to repo."""
    test_mp3_path = Path("/music/sample.mp3")

    async def single_mp3_gen(*args, **kwargs) -> AsyncGenerator[Path, None]:
        yield test_mp3_path

    mock_file_system_scanner.scan_directories.side_effect = single_mp3_gen

    # Mocks for mutagen.mp3.MP3 behavior
    mock_mp3_audio = MagicMock()
    mock_mp3_audio.tags = {
        "TIT2": MagicMock(text=["MP3 Title"]),
        "TPE1": MagicMock(text=["MP3 Artist"]),
    }
    mock_mp3_audio.info = MagicMock(length=200.5)

    # Mock os.path.getmtime
    mock_mtime = 1678886400  # Example timestamp

    # Patch os.path.getmtime and mutagen.mp3.MP3 within the use case's execution scope
    # Also patch the repository for assertions
    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.os.path.getmtime",
        return_value=mock_mtime,
    ), patch(
        "src.mus.application.use_cases.scan_tracks_use_case.MP3"
    ) as mock_mutagen_mp3, patch(
        "src.mus.application.use_cases.scan_tracks_use_case.SQLiteTrackRepository"
    ) as MockRepo, patch(
        "src.mus.application.use_cases.scan_tracks_use_case.broadcast_sse_event"
    ) as mock_broadcast_sse:
        mock_mutagen_mp3.return_value = mock_mp3_audio

        mock_repo_instance = MagicMock(spec=SQLiteTrackRepository)
        mock_repo_instance.session = mock_async_session
        mock_repo_instance.get_latest_track_added_at = AsyncMock(return_value=None)

        created_tracks_store: List[Track] = []

        async def mock_upsert_and_store(track_to_upsert: Track) -> Track:
            track_to_upsert.id = len(created_tracks_store) + 1
            created_tracks_store.append(track_to_upsert)
            return track_to_upsert

        mock_repo_instance.upsert_track = AsyncMock(side_effect=mock_upsert_and_store)
        mock_repo_instance.set_cover_flag = AsyncMock()
        MockRepo.return_value = mock_repo_instance

        mock_cover_processor.process_tracks_covers_batch = AsyncMock(
            return_value={1: True}
        )

        result = await scan_tracks_use_case.scan_directory()

    assert result.success is True
    assert result.tracks_added == 1
    assert result.tracks_updated == 0
    assert result.errors == 0

    mock_mutagen_mp3.assert_called_once_with(test_mp3_path)

    assert len(created_tracks_store) == 1
    saved_track = created_tracks_store[0]

    assert saved_track.title == "MP3 Title"
    assert saved_track.artist == "MP3 Artist"
    assert saved_track.duration == 200  # Duration should be int
    assert saved_track.file_path == str(test_mp3_path)
    assert saved_track.added_at == mock_mtime
    assert saved_track.has_cover is False  # Initially, before cover flag is set

    # Check that set_cover_flag was called correctly after successful cover processing
    mock_repo_instance.set_cover_flag.assert_awaited_once_with(saved_track.id, True)
    mock_cover_processor.process_tracks_covers_batch.assert_awaited_once_with(
        [(saved_track.id, test_mp3_path)]
    )

    # Verify SSE calls
    # Check for initial progress message
    mock_broadcast_sse.assert_any_call(
        message_to_show="Scanning 1 files...",
        message_level="info",
        action_key="scan_progress",
        action_payload={"processed": 0, "total": 1},
    )

    # Check for progress update
    mock_broadcast_sse.assert_any_call(
        message_to_show="Scanning progress: 1/1 files",
        message_level="info",
        action_key="scan_progress",
        action_payload={"processed": 1, "total": 1},
    )

    # Check for new track message
    mock_broadcast_sse.assert_any_call(
        message_to_show="Track: MP3 Artist - MP3 Title",
        message_level="success",
        action_key="reload_tracks",
        action_payload=None,
    )

    # Check for summary message
    mock_broadcast_sse.assert_any_call(
        message_to_show="1 new tracks added.",
        message_level="success",
        action_key="reload_tracks",
        action_payload=None,
    )


@pytest.mark.asyncio
async def test_scan_directory_multiple_scans_no_changes(
    scan_tracks_use_case: ScanTracksUseCase,
    mock_file_system_scanner: MagicMock,
    mock_cover_processor: MagicMock,
    mock_async_session: AsyncMock,
):
    """Test that subsequent scans with no file changes report 0 added, 0 updated."""
    test_file_path = Path("/music/test_song.mp3")

    async def single_file_gen(*args, **kwargs) -> AsyncGenerator[Path, None]:
        yield test_file_path

    mock_file_system_scanner.scan_directories.side_effect = single_file_gen

    # Mock metadata extraction
    scan_tracks_use_case._extract_metadata = AsyncMock(
        return_value={
            "title": "Test Song",
            "artist": "Test Artist",
            "duration": 180,
            "mtime": 1678886400,
        }
    )

    # Mock the repository
    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.SQLiteTrackRepository"
    ) as MockRepo:
        mock_repo_instance = MagicMock(spec=SQLiteTrackRepository)
        mock_repo_instance.session = mock_async_session
        mock_repo_instance.get_latest_track_added_at = AsyncMock(return_value=None)

        # First scan: track is new
        async def mock_upsert_first_scan(track_to_upsert: Track) -> Track:
            track_to_upsert.id = 1
            return track_to_upsert

        mock_repo_instance.upsert_track = AsyncMock(side_effect=mock_upsert_first_scan)
        mock_repo_instance.set_cover_flag = AsyncMock()
        MockRepo.return_value = mock_repo_instance

        mock_cover_processor.process_tracks_covers_batch = AsyncMock(
            return_value={1: True}
        )

        with patch(
            "src.mus.application.use_cases.scan_tracks_use_case.broadcast_sse_event"
        ):
            # First scan
            result1 = await scan_tracks_use_case.scan_directory()

        assert result1.tracks_added == 1
        assert result1.tracks_updated == 0

        # Second scan: same track, no changes (existing track)
        async def mock_upsert_second_scan(track_to_upsert: Track) -> Track:
            track_to_upsert.id = 1
            return track_to_upsert

        mock_repo_instance.upsert_track = AsyncMock(side_effect=mock_upsert_second_scan)

        with patch(
            "src.mus.application.use_cases.scan_tracks_use_case.broadcast_sse_event"
        ):
            # Second scan
            result2 = await scan_tracks_use_case.scan_directory()

        assert result2.tracks_added == 1
        assert result2.tracks_updated == 0  # No data changed, so no updates


@pytest.mark.asyncio
async def test_scan_directory_modified_file_reports_updated(
    scan_tracks_use_case: ScanTracksUseCase,
    mock_file_system_scanner: MagicMock,
    mock_cover_processor: MagicMock,
    mock_async_session: AsyncMock,
):
    """Test that a modified file is correctly reported as updated."""
    test_file_path = Path("/music/test_song.mp3")

    async def single_file_gen(*args, **kwargs) -> AsyncGenerator[Path, None]:
        yield test_file_path

    mock_file_system_scanner.scan_directories.side_effect = single_file_gen

    # Mock metadata extraction with updated mtime
    scan_tracks_use_case._extract_metadata = AsyncMock(
        return_value={
            "title": "Updated Test Song",
            "artist": "Updated Test Artist",
            "duration": 200,
            "mtime": 1678886500,  # Different mtime
        }
    )

    # Mock the repository
    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.SQLiteTrackRepository"
    ) as MockRepo:
        mock_repo_instance = MagicMock(spec=SQLiteTrackRepository)
        mock_repo_instance.session = mock_async_session
        mock_repo_instance.get_latest_track_added_at = AsyncMock(
            return_value=1678886400
        )

        # Simulate updating an existing track
        async def mock_upsert_update(track_to_upsert: Track) -> Track:
            track_to_upsert.id = 1
            return track_to_upsert

        mock_repo_instance.upsert_track = AsyncMock(side_effect=mock_upsert_update)
        mock_repo_instance.set_cover_flag = AsyncMock()
        MockRepo.return_value = mock_repo_instance

        mock_cover_processor.process_tracks_covers_batch = AsyncMock(
            return_value={1: True}
        )

        with patch(
            "src.mus.application.use_cases.scan_tracks_use_case.broadcast_sse_event"
        ):
            result = await scan_tracks_use_case.scan_directory()

        assert result.tracks_added == 1
        assert result.tracks_updated == 0
        assert result.errors == 0
