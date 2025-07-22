from pathlib import Path
from typing import Optional

from mutagen._file import File as MutagenFile
from mutagen.mp3 import MP3


class ID3TagResult:
    """Result of ID3 tag standardization operation."""

    def __init__(
        self,
        was_standardized: bool = False,
        old_version: Optional[tuple] = None,
        new_version: str = "2.3.0",
        encoding_upgraded: bool = False,
        error: Optional[str] = None,
    ):
        self.was_standardized = was_standardized
        self.old_version = old_version
        self.new_version = new_version
        self.encoding_upgraded = encoding_upgraded
        self.error = error


class ID3TagService:
    def standardize_file(self, file_path: Path) -> ID3TagResult:
        try:
            audio = MutagenFile(file_path, easy=False)
            if not isinstance(audio, MP3):
                return ID3TagResult()

            if not audio.tags:
                return ID3TagResult()

            needs_update, old_version, encoding_needs_upgrade = (
                self._check_needs_standardization(audio)
            )

            if not needs_update:
                return ID3TagResult()

            if old_version != (2, 3, 0):
                audio.save(v2_version=3)
            elif encoding_needs_upgrade:
                audio.save()

            return ID3TagResult(
                was_standardized=True,
                old_version=old_version,
                encoding_upgraded=encoding_needs_upgrade,
            )

        except Exception as e:
            return ID3TagResult(error=str(e))

    def analyze_file(self, audio) -> ID3TagResult:
        try:
            if not isinstance(audio, MP3):
                return ID3TagResult()

            if not audio.tags:
                return ID3TagResult()

            needs_update, old_version, encoding_needs_upgrade = (
                self._check_needs_standardization(audio)
            )

            if not needs_update:
                return ID3TagResult()

            return ID3TagResult(
                was_standardized=True,
                old_version=old_version,
                encoding_upgraded=encoding_needs_upgrade,
            )

        except Exception as e:
            return ID3TagResult(error=str(e))

    def _check_needs_standardization(
        self, audio: MP3
    ) -> tuple[bool, Optional[tuple], bool]:
        needs_update = False
        old_version = None
        encoding_needs_upgrade = False

        if audio.tags and audio.tags.version != (2, 3, 0):
            needs_update = True
            old_version = audio.tags.version

        if not needs_update and audio.tags:
            for tag in audio.tags.values():
                if tag.encoding not in [1, 2, 3]:  # 1=UTF-16, 2=UTF-16BE, 3=UTF-8
                    needs_update = True
                    encoding_needs_upgrade = True
                    break

        return needs_update, old_version, encoding_needs_upgrade
