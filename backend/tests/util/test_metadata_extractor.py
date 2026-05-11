from types import SimpleNamespace
from unittest.mock import patch

from src.mus.util.metadata_extractor import extract_fast_metadata


class FakeAudio(dict):
    info = SimpleNamespace(length=123)


def test_extract_fast_metadata_preserves_multiple_artists(tmp_path):
    file_path = tmp_path / "song.mp3"
    file_path.write_bytes(b"")
    audio = FakeAudio(title=["Song"], artist=["Artist One", "Artist Two"])

    with patch("src.mus.util.metadata_extractor.MutagenFile", return_value=audio):
        result = extract_fast_metadata(file_path)

    assert result is not None
    assert result["artist"] == "Artist One; Artist Two"
