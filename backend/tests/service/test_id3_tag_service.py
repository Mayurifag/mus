import pytest
from pathlib import Path
from unittest.mock import patch, MagicMock

from src.mus.service.id3_tag_service import (
    ID3TagService,
    ID3TagResult,
)
from src.mus.domain.entities.track import Track


@pytest.fixture
def service():
    """Create an ID3TagService instance."""
    return ID3TagService()


@pytest.fixture
def sample_track():
    """Create a sample track for testing."""
    return Track(
        id=1,
        title="Test Song",
        artist="Test Artist",
        duration=180,
        file_path="/test/path.mp3",
        added_at=1234567890,
        updated_at=1234567890,
        has_cover=False,
    )


@pytest.fixture
def mock_mp3_file():
    """Create a mock MP3 file object."""
    mock_audio = MagicMock()
    mock_audio.tags = MagicMock()
    mock_audio.tags.version = (2, 4, 0)
    mock_audio.tags.values.return_value = []
    mock_audio.save = MagicMock()
    return mock_audio


@pytest.fixture
def mock_non_mp3_file():
    """Create a mock non-MP3 file object."""
    # Create a mock that is not an instance of MP3
    mock_audio = MagicMock(spec=object)
    return mock_audio


class TestID3TagResult:
    """Test the ID3TagResult class."""

    def test_default_initialization(self):
        """Test default initialization of ID3TagResult."""
        result = ID3TagResult()
        assert result.was_standardized is False
        assert result.old_version is None
        assert result.new_version == "2.3.0"
        assert result.encoding_upgraded is False
        assert result.error is None

    def test_custom_initialization(self):
        """Test custom initialization of ID3TagResult."""
        result = ID3TagResult(
            was_standardized=True,
            old_version=(2, 4, 0),
            encoding_upgraded=True,
            error="Test error",
        )
        assert result.was_standardized is True
        assert result.old_version == (2, 4, 0)
        assert result.new_version == "2.3.0"
        assert result.encoding_upgraded is True
        assert result.error == "Test error"


class TestID3TagService:
    """Test the ID3TagService class."""

    @patch("src.mus.service.id3_tag_service.MutagenFile")
    def test_standardize_non_mp3_file(
        self, mock_mutagen_file, service, mock_non_mp3_file
    ):
        """Test that non-MP3 files are not processed."""
        mock_mutagen_file.return_value = mock_non_mp3_file

        result = service.standardize_file(Path("/test/file.txt"))
        assert result.was_standardized is False
        assert result.error is None

    @patch("src.mus.service.id3_tag_service.MutagenFile")
    def test_standardize_mp3_without_tags(self, mock_mutagen_file, service):
        """Test that MP3 files without tags are not processed."""
        from mutagen.mp3 import MP3

        mock_audio = MagicMock(spec=MP3)
        mock_audio.tags = None
        mock_mutagen_file.return_value = mock_audio

        result = service.standardize_file(Path("/test/file.mp3"))
        assert result.was_standardized is False
        assert result.error is None

    @patch("src.mus.service.id3_tag_service.MutagenFile")
    def test_standardize_already_v23_file(self, mock_mutagen_file, service):
        """Test that ID3v2.3 files are not re-processed."""
        from mutagen.mp3 import MP3

        mock_audio = MagicMock(spec=MP3)
        mock_audio.tags = MagicMock()
        mock_audio.tags.version = (2, 3, 0)
        mock_audio.tags.values.return_value = []
        mock_mutagen_file.return_value = mock_audio

        result = service.standardize_file(Path("/test/file.mp3"))
        assert result.was_standardized is False
        assert result.error is None

    @patch("src.mus.service.id3_tag_service.MutagenFile")
    def test_standardize_v24_to_v23(self, mock_mutagen_file, service):
        """Test standardizing ID3v2.4 to v2.3."""
        from mutagen.mp3 import MP3

        mock_audio = MagicMock(spec=MP3)
        mock_audio.tags = MagicMock()
        mock_audio.tags.version = (2, 4, 0)
        mock_audio.tags.values.return_value = []
        mock_audio.save = MagicMock()
        mock_mutagen_file.return_value = mock_audio

        result = service.standardize_file(Path("/test/file.mp3"))
        assert result.was_standardized is True
        assert result.old_version == (2, 4, 0)
        assert result.new_version == "2.3.0"

        # Verify save was called with correct version
        mock_audio.save.assert_called_once_with(v2_version=3)

    @patch("src.mus.service.id3_tag_service.MutagenFile")
    def test_standardize_v22_to_v23(self, mock_mutagen_file, service):
        """Test standardizing ID3v2.2 to v2.3."""
        from mutagen.mp3 import MP3

        mock_audio = MagicMock(spec=MP3)
        mock_audio.tags = MagicMock()
        mock_audio.tags.version = (2, 2, 0)
        mock_audio.tags.values.return_value = []
        mock_audio.save = MagicMock()
        mock_mutagen_file.return_value = mock_audio

        result = service.standardize_file(Path("/test/file.mp3"))
        assert result.was_standardized is True
        assert result.old_version == (2, 2, 0)
        assert result.new_version == "2.3.0"

        # Verify save was called with correct version
        mock_audio.save.assert_called_once_with(v2_version=3)

    @patch("src.mus.service.id3_tag_service.MutagenFile")
    def test_standardize_file_with_error(self, mock_mutagen_file, service):
        """Test handling of file errors."""
        mock_mutagen_file.side_effect = Exception("File not found")

        result = service.standardize_file(Path("/non/existent/file.mp3"))
        assert result.was_standardized is False
        assert result.error == "File not found"

    def test_check_needs_standardization_version(self, service):
        """Test the _check_needs_standardization method for version checking."""
        mock_audio = MagicMock()
        mock_audio.tags = MagicMock()
        mock_audio.tags.version = (2, 4, 0)
        mock_audio.tags.values.return_value = []

        needs_update, old_version, encoding_upgrade = (
            service._check_needs_standardization(mock_audio)
        )

        assert needs_update is True
        assert old_version == (2, 4, 0)
        assert encoding_upgrade is False

    def test_check_needs_standardization_no_update_needed(self, service):
        """Test the _check_needs_standardization method when no update is needed."""
        mock_audio = MagicMock()
        mock_audio.tags = MagicMock()
        mock_audio.tags.version = (2, 3, 0)
        mock_audio.tags.values.return_value = []

        needs_update, old_version, encoding_upgrade = (
            service._check_needs_standardization(mock_audio)
        )

        assert needs_update is False
        assert old_version is None
        assert encoding_upgrade is False

    def test_check_needs_standardization_encoding_upgrade(self, service):
        """Test the _check_needs_standardization method for encoding upgrade."""
        mock_audio = MagicMock()
        mock_audio.tags = MagicMock()
        mock_audio.tags.version = (2, 3, 0)

        # Create a mock tag with legacy encoding
        mock_tag = MagicMock()
        mock_tag.encoding = 0  # Legacy encoding
        mock_audio.tags.values.return_value = [mock_tag]

        needs_update, old_version, encoding_upgrade = (
            service._check_needs_standardization(mock_audio)
        )

        assert needs_update is True
        assert old_version is None
        assert encoding_upgrade is True
