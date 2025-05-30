from dataclasses import dataclass
from pathlib import Path


@dataclass
class Track:
    title: str
    artist: str
    duration: int  # Duration in seconds
    file_path: Path
    added_at: int  # Unix timestamp
    id: int | None = None
    has_cover: bool = False
    cover_small_url: str | None = None
    cover_medium_url: str | None = None
