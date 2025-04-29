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

            # Get basic metadata
            title = self._get_tag(audio, "title", file_path.stem)
            artist = self._get_tag(audio, "artist", "Unknown Artist")
            duration = int(audio.info.length) if hasattr(audio.info, "length") else 0
            added_at = int(file_path.stat().st_mtime)

            return (title, artist, duration, added_at)

        except Exception as e:
            logger.error(f"Error extracting metadata from {file_path}: {e!s}")
            return None

    def _get_tag(self, audio, tag_name: str, default: str) -> str:
        try:
            if hasattr(audio, "tags"):
                tags = audio.tags
                if isinstance(tags, dict) and tag_name in tags:
                    value = tags[tag_name][0]
                    return str(value) if value else default
                elif hasattr(tags, "__getitem__") and tag_name in tags:
                    value = tags[tag_name][0]
                    return str(value) if value else default
        except Exception:
            pass
        return default
