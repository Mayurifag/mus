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
        self.tag_mapping = {
            ".mp3": {"title": ["TIT2", "title"], "artist": ["TPE1", "artist"]},
            ".flac": {"title": ["title"], "artist": ["artist"]},
            ".ogg": {"title": ["title"], "artist": ["artist"]},
            ".m4a": {"title": ["©nam"], "artist": ["©ART"]},
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

            # Try to get metadata from tags first
            title = self._get_tag_for_format(audio, suffix, "title", None)
            artist = self._get_tag_for_format(audio, suffix, "artist", None)

            # If tags are missing or empty, try parsing filename
            if not title or not artist:
                title, artist = self._parse_filename(file_path)

            duration = self._get_duration_for_format(audio, suffix)
            added_at = int(file_path.stat().st_mtime)

            return (title, artist, duration, added_at)

        except Exception as e:
            logger.error(f"Error extracting metadata from {file_path}: {e!s}")
            return None

    def _get_tag_for_format(
        self, audio, suffix: str, tag_name: str, default: str | None
    ) -> str | None:
        try:
            if suffix in self.supported_formats and audio.tags:
                for tag_key in self.tag_mapping[suffix][tag_name]:
                    if tag_key in audio.tags:
                        value = audio.tags[tag_key][0]
                        if value:
                            return str(value)
        except Exception:
            pass
        return default

    def _parse_filename(self, file_path: Path) -> tuple[str, str]:
        stem = file_path.stem
        if " - " in stem:
            parts = stem.rsplit(" - ", 1)
            return parts[1].strip(), parts[0].strip()
        return stem, "Unknown Artist"

    def _get_duration_for_format(self, audio, suffix: str) -> int:
        try:
            if suffix in self.supported_formats:
                return int(audio.info.length)
        except Exception:
            pass
        return 0
