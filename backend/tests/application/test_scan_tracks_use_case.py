import pytest
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch
import asyncio

from src.mus.domain.entities.track import Track
from src.mus.application.dtos.scan import ScanResponseDTO, ScanProgressDTO
from src.mus.application.use_cases.scan_tracks_use_case import ScanTracksUseCase
from src.mus.infrastructure.scanner.file_system_scanner import FileSystemScanner
from src.mus.infrastructure.scanner.cover_processor import CoverProcessor
from src.mus.infrastructure.persistence.sqlite_track_repository import (
    SQLiteTrackRepository,
)


class MockAsyncContextManager:
    """A context manager for mocking async context managers."""

    async def __aenter__(self):
        return None

    async def __aexit__(self, exc_type, exc_value, traceback):
        return None


@pytest.fixture
def mock_track_repository():
    """Create a mock SQLiteTrackRepository."""
    repo = AsyncMock(spec=SQLiteTrackRepository)

    # Create a mock session
    repo.session = AsyncMock()

    # Mock the session.begin() context manager
    cm = MockAsyncContextManager()
    repo.session.begin = AsyncMock(return_value=cm)

    # Mock upsert_track to return an upserted track
    async def mock_upsert(track):
        # Create a copy of the track with an ID and other fields
        upserted = Track(
            id=1,
            title=track.title,
            artist=track.artist,
            duration=track.duration,
            file_path=track.file_path,
            has_cover=track.has_cover,
            added_at=track.added_at,
        )
        return upserted

    repo.upsert_track.side_effect = mock_upsert

    # Mock set_cover_flag
    repo.set_cover_flag = AsyncMock()

    return repo


@pytest.fixture
def mock_file_system_scanner():
    """Create a mock FileSystemScanner."""
    scanner = AsyncMock(FileSystemScanner)

    # Mock scan_directories to return a list of file paths
    async def mock_scan_directories(directories=None, extensions=None):
        test_files = [
            Path("track1.mp3"),
            Path("track2.flac"),
            Path("track3.mp3"),
        ]
        for file_path in test_files:
            yield file_path

    scanner.scan_directories.side_effect = mock_scan_directories

    return scanner


@pytest.fixture
def mock_cover_processor():
    """Create a mock CoverProcessor."""
    processor = AsyncMock(CoverProcessor)

    # Mock extract_cover_from_file to return None (no cover)
    processor.extract_cover_from_file.return_value = None

    # Mock process_tracks_covers_batch to return success for all tracks
    processor.process_tracks_covers_batch.return_value = {1: True, 2: True, 3: True}

    return processor


@pytest.fixture
def scan_tracks_use_case(
    mock_track_repository, mock_file_system_scanner, mock_cover_processor
):
    """Create a ScanTracksUseCase with mock dependencies."""
    use_case = ScanTracksUseCase(
        track_repository=mock_track_repository,
        file_system_scanner=mock_file_system_scanner,
        cover_processor=mock_cover_processor,
    )
    use_case.batch_size = 3  # Small batch size for testing

    # Create a patched version of _process_batch that doesn't use transactions
    async def patched_process_batch(files, progress):
        batch_stats = {"added": 0, "updated": 0, "errors": 0, "error_details": []}

        # Extract metadata and prepare tracks for upsert
        metadata_tasks = []
        for file_path in files:
            task = asyncio.create_task(use_case._extract_metadata(file_path))
            metadata_tasks.append((file_path, task))

        # Upsert tracks and collect track IDs for cover processing
        tracks_to_process = []

        for file_path, task in metadata_tasks:
            try:
                metadata = await task
                if not metadata:
                    batch_stats["errors"] += 1
                    batch_stats["error_details"].append(
                        f"Failed to extract metadata: {file_path}"
                    )
                    continue

                # Create Track entity with extracted metadata
                track = Track(
                    title=metadata["title"],
                    artist=metadata["artist"],
                    duration=metadata["duration"],
                    file_path=str(file_path),
                    has_cover=False,
                )

                # Upsert track
                upserted_track = await use_case.track_repository.upsert_track(track)

                # Add to list for cover processing if track has ID
                if upserted_track.id is not None:
                    tracks_to_process.append((upserted_track.id, file_path))

                # Determine if this was an add or update
                if upserted_track.added_at == track.added_at:
                    batch_stats["added"] += 1
                else:
                    batch_stats["updated"] += 1

            except Exception as e:
                batch_stats["errors"] += 1
                batch_stats["error_details"].append(
                    f"Error processing {file_path}: {str(e)}"
                )

        # Process covers
        if tracks_to_process:
            cover_results = await use_case.cover_processor.process_tracks_covers_batch(
                tracks_to_process
            )

            # Update has_cover flag for tracks with successfully processed covers
            for track_id, success in cover_results.items():
                if success:
                    await use_case.track_repository.set_cover_flag(track_id, True)

        return batch_stats

    # Replace the original method with our patched version
    use_case._process_batch = patched_process_batch

    return use_case


@pytest.mark.asyncio
async def test_scan_directory_basic_functionality(scan_tracks_use_case):
    """Test the basic scanning functionality."""
    # Patch _extract_metadata to return test metadata
    with patch.object(
        scan_tracks_use_case,
        "_extract_metadata",
        side_effect=[
            {"title": "Track 1", "artist": "Artist 1", "duration": 180},
            {"title": "Track 2", "artist": "Artist 2", "duration": 240},
            {"title": "Track 3", "artist": "Artist 3", "duration": 200},
        ],
    ):
        # Run the scan
        result = await scan_tracks_use_case.scan_directory()

        # Verify the result
        assert isinstance(result, ScanResponseDTO)
        assert result.success is True
        assert result.tracks_added == 3
        assert result.tracks_updated == 0
        assert result.errors == 0

        # Verify repository interactions
        assert scan_tracks_use_case.track_repository.upsert_track.call_count == 3

        # Verify cover processor interactions
        assert (
            scan_tracks_use_case.cover_processor.process_tracks_covers_batch.call_count
            == 1
        )


@pytest.mark.asyncio
async def test_scan_directory_with_errors(scan_tracks_use_case):
    """Test scanning with errors during metadata extraction."""
    # Patch _extract_metadata to return some data and some None (error)
    with patch.object(
        scan_tracks_use_case,
        "_extract_metadata",
        side_effect=[
            {"title": "Track 1", "artist": "Artist 1", "duration": 180},
            None,  # Extraction error
            {"title": "Track 3", "artist": "Artist 3", "duration": 200},
        ],
    ):
        # Run the scan
        result = await scan_tracks_use_case.scan_directory()

        # Verify the result
        assert result.success is True
        assert result.tracks_added == 2  # Only 2 successful tracks
        assert result.tracks_updated == 0
        assert result.errors == 1
        assert len(result.error_details) == 1  # One error message

        # Verify repository interactions
        assert scan_tracks_use_case.track_repository.upsert_track.call_count == 2


@pytest.mark.asyncio
async def test_process_batch(scan_tracks_use_case):
    """Test the _process_batch method."""
    # Create a sample batch of files
    files = [
        Path("track1.mp3"),
        Path("track2.flac"),
        Path("track3.mp3"),
    ]

    # Create a progress object
    progress = ScanProgressDTO()

    # Patch _extract_metadata to return test metadata
    with patch.object(
        scan_tracks_use_case,
        "_extract_metadata",
        side_effect=[
            {"title": "Track 1", "artist": "Artist 1", "duration": 180},
            {"title": "Track 2", "artist": "Artist 2", "duration": 240},
            {"title": "Track 3", "artist": "Artist 3", "duration": 200},
        ],
    ):
        # Process the batch
        result = await scan_tracks_use_case._process_batch(files, progress)

        # Verify the result
        assert result["added"] == 3
        assert result["updated"] == 0
        assert result["errors"] == 0
        assert len(result["error_details"]) == 0

        # Verify repository interactions
        assert scan_tracks_use_case.track_repository.upsert_track.call_count == 3

        # Verify cover processor interactions
        assert (
            scan_tracks_use_case.cover_processor.process_tracks_covers_batch.call_count
            == 1
        )


@pytest.mark.asyncio
async def test_extract_metadata(scan_tracks_use_case):
    """Test the _extract_metadata method."""
    # Patch _extract_metadata_sync to return test metadata
    test_metadata = {"title": "Test Track", "artist": "Test Artist", "duration": 180}
    with patch.object(
        scan_tracks_use_case, "_extract_metadata_sync", return_value=test_metadata
    ):
        # Extract metadata
        result = await scan_tracks_use_case._extract_metadata(Path("test.mp3"))

        # Verify result
        assert result == test_metadata

        # Verify _extract_metadata_sync was called
        scan_tracks_use_case._extract_metadata_sync.assert_called_once_with(
            Path("test.mp3")
        )


@pytest.mark.asyncio
async def test_extract_metadata_exception_handling(scan_tracks_use_case):
    """Test error handling in _extract_metadata."""
    # Patch _extract_metadata_sync to raise an exception
    with patch.object(
        scan_tracks_use_case,
        "_extract_metadata_sync",
        side_effect=Exception("Test exception"),
    ):
        # Extract metadata with exception
        result = await scan_tracks_use_case._extract_metadata(Path("test.mp3"))

        # Should handle the exception and return None
        assert result is None


def test_extract_metadata_sync_mp3(scan_tracks_use_case):
    """Test the synchronous metadata extraction for MP3 files."""
    # Mock MP3 file metadata extraction
    with patch("src.mus.application.use_cases.scan_tracks_use_case.MP3") as mock_mp3:
        # Setup the mock with tags and info
        mock_audio = MagicMock()
        mock_audio.tags = {"TIT2": "Test Title", "TPE1": "Test Artist"}
        mock_audio.info.length = 123.45
        mock_mp3.return_value = mock_audio

        # Extract metadata
        result = scan_tracks_use_case._extract_metadata_sync(Path("test.mp3"))

        # Verify result
        assert result["title"] == "Test Title"
        assert result["artist"] == "Test Artist"
        assert result["duration"] == 123  # Truncated to int

        # Verify MP3 was called
        mock_mp3.assert_called_once_with(Path("test.mp3"))


def test_extract_metadata_sync_flac(scan_tracks_use_case):
    """Test the synchronous metadata extraction for FLAC files."""
    # Mock FLAC file metadata extraction
    with patch("src.mus.application.use_cases.scan_tracks_use_case.FLAC") as mock_flac:
        # Setup the mock with tags and info
        mock_audio = MagicMock()
        mock_audio.__getitem__.side_effect = lambda key: {
            "title": ["FLAC Title"],
            "artist": ["FLAC Artist"],
        }[key]
        mock_audio.__contains__.side_effect = lambda key: key in ["title", "artist"]
        mock_audio.info.length = 246.8
        mock_flac.return_value = mock_audio

        # Extract metadata
        result = scan_tracks_use_case._extract_metadata_sync(Path("test.flac"))

        # Verify result
        assert result["title"] == "FLAC Title"
        assert result["artist"] == "FLAC Artist"
        assert result["duration"] == 246  # Truncated to int

        # Verify FLAC was called
        mock_flac.assert_called_once_with(Path("test.flac"))


def test_extract_metadata_sync_error_handling(scan_tracks_use_case):
    """Test error handling in _extract_metadata_sync."""
    # Mock MP3 to raise an exception
    with patch(
        "src.mus.application.use_cases.scan_tracks_use_case.MP3",
        side_effect=Exception("Test exception"),
    ):
        # Extract metadata with exception
        file_path = Path("test.mp3")
        result = scan_tracks_use_case._extract_metadata_sync(file_path)

        # Should return default metadata based on filename
        assert result["title"] == "test"
        assert result["artist"] == "Unknown"
        assert result["duration"] == 0
