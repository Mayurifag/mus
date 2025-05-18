from sqlmodel import Field, SQLModel
from typing import Optional


class Track(SQLModel, table=True):
    """Track entity representing a music file."""

    id: Optional[int] = Field(default=None, primary_key=True)
    title: str
    artist: str
    duration: int  # Duration in seconds
    file_path: str = Field(unique=True, index=True)
    added_at: int  # Unix timestamp from file mtime
    has_cover: bool = Field(default=False)
