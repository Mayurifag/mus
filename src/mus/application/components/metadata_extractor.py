import logging
from pathlib import Path

from mutagen.flac import FLAC
from mutagen.mp3 import MP3
from mutagen.mp4 import MP4
from mutagen.oggvorbis import OggVorbis

from mus.application.ports.metadata_reader import IMetadataReader

logger = logging.getLogger(__name__)


class MetadataExtractor(IMetadataReader):
    def __init__(self):
        self.supported_formats = {
            ".mp3": MP3,
            ".flac": FLAC,
            ".ogg": OggVorbis,
            ".m4a": MP4,
        }

    async def read_metadata(self, file_path: Path) -> tuple[str, str, int, int] | None:
        try:
            suffix = file_path.suffix.lower()
            if suffix not in self.supported_formats:
                logger.warning(f"Unsupported file format: {file_path}")
                return None

            format_class = self.supported_formats[suffix]
            audio = format_class(str(file_path))
            if audio is None:
                logger.warning(f"Could not read metadata from {file_path}")
                return None

            # Get basic metadata based on file format
            title = self._get_tag_for_format(audio, suffix, "title", file_path.stem)
            artist = self._get_tag_for_format(audio, suffix, "artist", "Unknown Artist")
            duration = self._get_duration_for_format(audio, suffix)
            added_at = int(file_path.stat().st_mtime)

            return (title, artist, duration, added_at)

        except Exception as e:
            logger.error(f"Error extracting metadata from {file_path}: {e!s}")
            return None

    def _get_tag_for_format(
        self, audio, suffix: str, tag_name: str, default: str
    ) -> str:
        try:
            if (
                suffix in self.supported_formats
                and audio.tags
                and tag_name in audio.tags
            ):
                value = audio.tags[tag_name][0]
                return str(value) if value else default
        except Exception:
            pass
        return default

    def _get_duration_for_format(self, audio, suffix: str) -> int:
        try:
            if suffix in self.supported_formats:
                return int(audio.info.length)
        except Exception:
            pass
        return 0
