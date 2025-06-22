from pathlib import Path
import ffmpeg


def get_accurate_duration(file_path: Path) -> int:
    try:
        probe = ffmpeg.probe(str(file_path))
        duration = float(probe["format"]["duration"])
        return int(duration)
    except Exception:
        return 0
