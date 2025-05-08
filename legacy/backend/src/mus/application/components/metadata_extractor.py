import base64
import logging
from pathlib import Path

from mutagen.flac import FLAC, Picture
from mutagen.id3 import ID3
from mutagen.mp3 import MP3
from mutagen.mp4 import MP4, MP4Cover
from mutagen.oggvorbis import OggVorbis

from mus.application.ports.metadata_reader import IMetadataReader

logger = logging.getLogger(__name__)

FRONT_COVER_TYPE = 3  # Standard type for front cover in ID3v2


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

    async def read_metadata(
        self, file_path: Path
    ) -> tuple[str, str, int, int, bytes | None] | None:
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
            cover_data = self._extract_cover_art(audio, suffix, file_path)

            return (title, artist, duration, added_at, cover_data)

        except Exception as e:
            logger.error(f"Error extracting metadata from {file_path}: {e!s}")
            return None

    def _extract_cover_art(self, audio, suffix: str, file_path: Path) -> bytes | None:
        try:
            if suffix == ".mp3":
                return self._extract_mp3_cover(file_path)
            elif suffix == ".flac":
                return self._extract_flac_cover(audio)
            elif suffix == ".m4a":
                return self._extract_m4a_cover(audio)
            elif suffix == ".ogg":
                return self._extract_ogg_cover(audio)
            return None
        except Exception as e:
            logger.warning(f"Error extracting cover art: {e!s}")
            return None

    def _extract_mp3_cover(self, file_path: Path) -> bytes | None:
        try:
            id3 = ID3(str(file_path))
            for key in id3.keys():
                if key.startswith("APIC"):
                    return id3[key].data
        except Exception:
            pass
        return None

    def _extract_flac_cover(self, audio) -> bytes | None:
        if not audio.pictures:
            return None
        # Try to find front cover first
        for pic in audio.pictures:
            if isinstance(pic, Picture) and pic.type == FRONT_COVER_TYPE:
                return pic.data
        # Fall back to first picture if no front cover
        return audio.pictures[0].data

    def _extract_m4a_cover(self, audio) -> bytes | None:
        if "covr" in audio:
            cover = audio["covr"][0]
            if isinstance(cover, MP4Cover):
                return bytes(cover)
        return None

    def _extract_ogg_cover(self, audio) -> bytes | None:
        if audio.tags and "metadata_block_picture" in audio.tags:
            pic = Picture(base64.b64decode(audio.tags["metadata_block_picture"][0]))
            return pic.data
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
