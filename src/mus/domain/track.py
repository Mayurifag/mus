from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from uuid import UUID


@dataclass
class Track:
    id: UUID
    file_path: Path
    title: str
    artist: str
    duration: float  # Duration in seconds
    added_at: datetime
