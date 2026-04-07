import pytest

from src.mus.domain.services.progress_parser import parse_progress_line


@pytest.mark.parametrize(
    "line, expected",
    [
        # Standard in-progress line with ~ size prefix
        (
            "[download]   42.3% of ~12.34MiB at  1.23MiB/s ETA 00:07",
            {"percent": 42.3, "speed": "1.23MiB/s", "eta": "00:07"},
        ),
        # 0% initial line
        (
            "[download]    0.0% of ~50.00MiB at  Unknown B/s ETA Unknown",
            {"percent": 0.0, "speed": "Unknown B/s", "eta": "Unknown"},
        ),
        # Line with Unknown speed
        (
            "[download]   15.0% of ~8.00MiB at  Unknown B/s ETA 00:30",
            {"percent": 15.0, "speed": "Unknown B/s", "eta": "00:30"},
        ),
        # Line with Unknown ETA
        (
            "[download]   75.5% of ~6.00MiB at  2.00MiB/s ETA Unknown",
            {"percent": 75.5, "speed": "2.00MiB/s", "eta": "Unknown"},
        ),
        # 100% completion line (different format: "in HH:MM:SS")
        (
            "[download] 100% of   8.45MiB in 00:00:03",
            {"percent": 100.0, "speed": "", "eta": ""},
        ),
        # Non-progress line returns None
        (
            "[info] Extracting URL: https://www.youtube.com/watch?v=abc",
            None,
        ),
        # Empty string returns None
        (
            "",
            None,
        ),
        # [ExtractAudio] line returns None
        (
            "[ExtractAudio] Destination: /tmp/Artist - Track.mp3",
            None,
        ),
        # [download] Destination: line returns None
        (
            "[download] Destination: /tmp/Artist - Track.webm",
            None,
        ),
    ],
)
def test_parse_progress_line(line: str, expected: dict | None):
    assert parse_progress_line(line) == expected
